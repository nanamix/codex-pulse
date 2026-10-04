//! Test-only App Server peer. No network or real account access.
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
fn emit(value: Value) {
    println!("{value}");
    io::stdout().flush().unwrap();
}
fn main() {
    let mode = std::env::var("MOCK_MODE").unwrap_or_default();
    let mut initialized = false;
    let mut count = 0;
    let mut rate_calls = 0;
    for line in io::stdin().lock().lines() {
        let req: Value = serde_json::from_str(&line.unwrap()).unwrap();
        let method = req["method"].as_str().unwrap_or("");
        if method == "initialized" {
            initialized = true;
            continue;
        }
        if req.get("id").is_none() {
            continue;
        }
        let id = req["id"].clone();
        if method != "initialize" && !initialized {
            emit(json!({"id":id,"error":{"code":-32000,"message":"not initialized"}}));
            continue;
        }
        let result = match method {
            "initialize" => json!({"userAgent":"mock"}),
            "account/read" => {
                count += 1;
                if mode == "startup-account-notify" {
                    emit(json!({"method":"account/updated","params":{"authMode":"chatgpt"}}));
                }
                let account = if mode == "login" {
                    Value::Null
                } else {
                    json!({"type":if mode == "apikey" {"apiKey"} else {"chatgpt"},"email":if mode == "account-change" && count > 1 {"second@example.invalid"} else {"first@example.invalid"}})
                };
                json!({"account":account,"requiresOpenaiAuth":true})
            }
            "account/rateLimits/read" => {
                rate_calls += 1;
                if mode == "mid-read-account-notify" {
                    emit(json!({"method":"account/updated","params":{"authMode":"chatgpt"}}));
                }
                if mode == "slow-read" {
                    std::thread::sleep(std::time::Duration::from_millis(250));
                }
                if mode == "timeout" && count > 1 {
                    std::thread::sleep(std::time::Duration::from_secs(30));
                }
                if mode == "account-change" && count > 1 {
                    emit(
                        json!({"id":id,"error":{"code":-32001,"message":"secret-token-never-log"}}),
                    );
                    continue;
                }
                let mut bucket = json!({"limitId":"codex","primary":{"usedPercent":25,"windowDurationMins":15,"resetsAt":1730947200},"secondary":null});
                let mut notification = bucket.clone();
                notification["primary"]["usedPercent"] = json!(31);
                emit(
                    json!({"method":"account/rateLimits/updated","params":{"rateLimits":notification}}),
                );
                emit(json!({"id":99999,"result":{"wrong":true}}));
                if mode == "slow-read" {
                    bucket["credits"] = json!({"balance":rate_calls.to_string()});
                }
                let credits = if req["params"]["excludeResetCreditDetails"] == false {
                    json!([{"status":"available","expiresAt":1730947200}])
                } else {
                    Value::Null
                };
                let mut result = json!({"rateLimits":bucket,"rateLimitResetCredits":{"availableCount":7,"credits":credits}});
                if mode == "workspace-change" {
                    result["accountId"] = json!(if count == 1 {
                        "workspace-a"
                    } else {
                        "workspace-b"
                    });
                }
                result
            }
            "account/usage/read" => {
                if mode == "post-read-notify" {
                    emit(
                        json!({"method":"account/rateLimits/updated","params":{"rateLimits":{"limitId":"codex","primary":{"usedPercent":31,"windowDurationMins":15}}}}),
                    );
                }
                if mode == "workspace-change" {
                    json!({"summary":{"lifetimeTokens":count*100}})
                } else {
                    emit(json!({"id":id,"error":{"code":-32601,"message":"unsupported"}}));
                    continue;
                }
            }
            "mock/error" => {
                emit(json!({"id":id,"error":{"code":-32000,"message":"secret-token-never-log"}}));
                continue;
            }
            _ => json!({}),
        };
        emit(json!({"id":id,"result":result}));
    }
}

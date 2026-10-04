#[path = "../../../tests/support/mod.rs"]
mod support;
use std::process::Command;
fn server() -> String {
    support::server().to_string_lossy().into_owned()
}
#[test]
fn one_shot_json_is_parseable() {
    let out = Command::new(env!("CARGO_BIN_EXE_codex-usage"))
        .args(["--json", "--codex-path", &server()])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["snapshot"]["remainingResetCount"].as_u64(), Some(7));
    assert_eq!(
        v["snapshot"]["resetCreditExpirations"],
        serde_json::json!([1730947200])
    );
    assert_eq!(
        v["snapshot"]["buckets"][0]["primary"]["usedPercent"].as_f64(),
        Some(25.0)
    );
}
#[test]
fn login_failure_exits_one_without_secret_output() {
    let out = Command::new(env!("CARGO_BIN_EXE_codex-usage"))
        .env("MOCK_MODE", "login")
        .args(["--json", "--codex-path", &server()])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&out.stderr).contains("secret"));
}
#[test]
fn invalid_interval_is_argument_error() {
    let out = Command::new(env!("CARGO_BIN_EXE_codex-usage"))
        .args(["--interval", "0"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
}

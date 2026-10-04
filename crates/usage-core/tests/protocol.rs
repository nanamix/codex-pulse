#[path = "../../../tests/support/mod.rs"]
mod support;
use std::{path::PathBuf, time::Duration};
use usage_core::{Client, UsageError};
fn executable() -> PathBuf {
    support::server()
}
#[tokio::test]
async fn initializes_and_matches_ids_across_notifications() {
    let mut c = Client::connect(&executable(), Duration::from_secs(2))
        .await
        .unwrap();
    let result = c
        .request("account/rateLimits/read", serde_json::json!({}))
        .await
        .unwrap();
    assert_eq!(result["rateLimits"]["primary"]["usedPercent"], 25);
    assert_eq!(
        c.drain_notifications()[0]["params"]["rateLimits"]["primary"]["usedPercent"],
        31
    );
    #[cfg(unix)]
    let pid = c.pid().unwrap();
    c.close().await;
    assert!(c.pid().is_none());
    #[cfg(unix)]
    assert!(!std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .unwrap()
        .success());
}
#[tokio::test]
async fn rpc_errors_never_expose_server_message() {
    let mut c = Client::connect(&executable(), Duration::from_secs(2))
        .await
        .unwrap();
    let e = c
        .request("mock/error", serde_json::json!({}))
        .await
        .unwrap_err();
    assert!(matches!(e, UsageError::Rpc(-32000)));
    assert!(!e.to_string().contains("secret"));
    c.close().await;
}

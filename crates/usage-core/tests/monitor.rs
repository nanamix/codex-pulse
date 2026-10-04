#[path = "../../../tests/support/mod.rs"]
mod support;
#[cfg(unix)]
use std::path::PathBuf;
use std::time::Duration;
use usage_core::{MonitorConfig, MonitorHandle, Status};
fn config() -> MonitorConfig {
    MonitorConfig {
        codex_path: support::server(),
        interval_secs: 30,
    }
}
#[tokio::test]
async fn unsupported_token_usage_keeps_limits_and_shutdown_stops() {
    let monitor = MonitorHandle::start(config()).unwrap();
    let mut rx = monitor.subscribe();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if rx.borrow().status == Status::Ready {
                break;
            }
            rx.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    let state = rx.borrow().clone();
    assert!(!state.stale);
    assert_eq!(
        state.snapshot.as_ref().unwrap().remaining_reset_count,
        Some(7)
    );
    assert_eq!(
        state.snapshot.as_ref().unwrap().reset_credit_expirations,
        Some(vec![Some(1730947200)])
    );
    assert!(state.snapshot.as_ref().unwrap().token_summary.is_none());
    assert!(state.snapshot.as_ref().unwrap().buckets[0]
        .primary
        .is_some());
    monitor.shutdown().await;
    assert_eq!(rx.borrow().status, Status::Stopped);
}
#[test]
fn rejects_invalid_configuration() {
    let mut c = config();
    c.interval_secs = 0;
    assert!(MonitorHandle::start(c).is_err());
}

#[cfg(unix)]
fn mode_config(mode: &str) -> (MonitorConfig, PathBuf) {
    use std::os::unix::fs::PermissionsExt;
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "codex-mock-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let server = config().codex_path;
    let body = format!(
        "#!/bin/sh\nMOCK_MODE='{}' exec '{}' \"$@\"\n",
        mode,
        server.to_string_lossy().replace('\'', "'\\''")
    );
    std::fs::write(&path, body).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    (
        MonitorConfig {
            codex_path: path.clone(),
            interval_secs: 30,
        },
        path,
    )
}
#[cfg(unix)]
async fn wait_status(
    rx: &mut tokio::sync::watch::Receiver<usage_core::MonitorState>,
    status: Status,
) -> usage_core::MonitorState {
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            let state = rx.borrow_and_update().clone();
            if state.status == status {
                return state;
            }
            rx.changed().await.unwrap();
        }
    })
    .await
    .unwrap()
}
#[cfg(unix)]
#[tokio::test]
async fn account_read_supersedes_startup_account_notification() {
    let (config, wrapper) = mode_config("startup-account-notify");
    let monitor = MonitorHandle::start(config).unwrap();
    let mut rx = monitor.subscribe();
    let result = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let state = rx.borrow_and_update().clone();
            if state.status == Status::Ready {
                break state;
            }
            rx.changed().await.unwrap();
        }
    })
    .await;
    monitor.shutdown().await;
    std::fs::remove_file(wrapper).unwrap();
    assert!(
        result.is_ok(),
        "startup notification must not force reconnect after authoritative account read"
    );
}
#[cfg(unix)]
#[tokio::test]
async fn account_notification_during_limits_read_prevents_publishing_snapshot() {
    let (config, wrapper) = mode_config("mid-read-account-notify");
    let monitor = MonitorHandle::start(config).unwrap();
    let mut rx = monitor.subscribe();
    let state = wait_status(&mut rx, Status::Error).await;
    assert!(state.snapshot.is_none());
    monitor.shutdown().await;
    std::fs::remove_file(wrapper).unwrap();
}
#[cfg(unix)]
#[tokio::test]
async fn clears_snapshot_on_account_change_even_if_new_read_fails() {
    let (config, wrapper) = mode_config("account-change");
    let monitor = MonitorHandle::start(config).unwrap();
    let mut rx = monitor.subscribe();
    wait_status(&mut rx, Status::Ready).await;
    monitor.refresh().await.unwrap();
    let state = wait_status(&mut rx, Status::Error).await;
    assert!(state.snapshot.is_none());
    assert!(!state.stale);
    monitor.shutdown().await;
    std::fs::remove_file(wrapper).unwrap();
}
#[cfg(unix)]
#[tokio::test]
async fn retains_stale_snapshot_on_timeout() {
    let (config, wrapper) = mode_config("timeout");
    let monitor = MonitorHandle::start(config).unwrap();
    let mut rx = monitor.subscribe();
    wait_status(&mut rx, Status::Ready).await;
    monitor.refresh().await.unwrap();
    let state = wait_status(&mut rx, Status::Error).await;
    assert!(state.snapshot.is_some());
    assert!(state.stale);
    assert!(state.message.unwrap().contains("제한시간"));
    monitor.shutdown().await;
    std::fs::remove_file(wrapper).unwrap();
}
#[cfg(unix)]
#[tokio::test]
async fn shutdown_interrupts_a_pending_request() {
    let (config, wrapper) = mode_config("timeout");
    let monitor = MonitorHandle::start(config).unwrap();
    let mut rx = monitor.subscribe();
    wait_status(&mut rx, Status::Ready).await;
    monitor.refresh().await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    tokio::time::timeout(Duration::from_secs(2), monitor.shutdown())
        .await
        .unwrap();
    assert_eq!(rx.borrow().status, Status::Stopped);
    std::fs::remove_file(wrapper).unwrap();
}
#[cfg(unix)]
#[tokio::test]
async fn notification_after_limit_read_is_not_discarded_by_optional_usage_read() {
    let (config, wrapper) = mode_config("post-read-notify");
    let monitor = MonitorHandle::start(config).unwrap();
    let mut rx = monitor.subscribe();
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let state = rx.borrow_and_update().clone();
            if state
                .snapshot
                .as_ref()
                .and_then(|s| s.buckets[0].primary.as_ref())
                .is_some_and(|w| w.used_percent == 31.0)
            {
                break;
            }
            rx.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    monitor.shutdown().await;
    std::fs::remove_file(wrapper).unwrap();
}
#[cfg(unix)]
#[tokio::test]
async fn refresh_burst_does_not_cancel_or_duplicate_the_in_flight_read() {
    let (config, wrapper) = mode_config("slow-read");
    let monitor = MonitorHandle::start(config).unwrap();
    let mut rx = monitor.subscribe();
    for _ in 0..20 {
        monitor.refresh().await.unwrap();
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    let state = wait_status(&mut rx, Status::Ready).await;
    assert_eq!(
        state.snapshot.unwrap().buckets[0]
            .credits
            .as_ref()
            .unwrap()
            .balance
            .as_deref(),
        Some("1")
    );
    monitor.shutdown().await;
    std::fs::remove_file(wrapper).unwrap();
}
#[cfg(unix)]
#[tokio::test]
async fn same_email_workspace_change_does_not_reuse_previous_token_summary() {
    let (config, wrapper) = mode_config("workspace-change");
    let monitor = MonitorHandle::start(config).unwrap();
    let mut rx = monitor.subscribe();
    let first = wait_status(&mut rx, Status::Ready).await;
    assert_eq!(
        first
            .snapshot
            .unwrap()
            .token_summary
            .unwrap()
            .lifetime_tokens,
        Some(100)
    );
    monitor.refresh().await.unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            rx.changed().await.unwrap();
            let state = rx.borrow_and_update().clone();
            if state.status == Status::Ready {
                assert_eq!(
                    state
                        .snapshot
                        .unwrap()
                        .token_summary
                        .unwrap()
                        .lifetime_tokens,
                    Some(200)
                );
                break;
            }
        }
    })
    .await
    .unwrap();
    monitor.shutdown().await;
    std::fs::remove_file(wrapper).unwrap();
}

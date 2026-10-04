use crate::{
    now, parse_limits, parse_reset_count, parse_reset_expirations, Client, MonitorState, Status,
    UsageError, UsageSnapshot,
};
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::{
    sync::{mpsc, oneshot, watch, Notify},
    time::Instant,
};

#[derive(Debug, Clone)]
pub struct MonitorConfig {
    pub codex_path: PathBuf,
    pub interval_secs: u64,
}
impl MonitorConfig {
    pub fn validate(&self) -> Result<(), UsageError> {
        if self.codex_path.as_os_str().is_empty() || !(5..=3600).contains(&self.interval_secs) {
            return Err(UsageError::InvalidConfig);
        }
        Ok(())
    }
}
enum Control {
    Configure(MonitorConfig, oneshot::Sender<()>),
    Stop(oneshot::Sender<()>),
}
#[derive(Clone)]
pub struct MonitorHandle {
    commands: mpsc::Sender<Control>,
    states: watch::Receiver<MonitorState>,
    refresh: Arc<Notify>,
}
impl MonitorHandle {
    pub fn start(config: MonitorConfig) -> Result<Self, UsageError> {
        config.validate()?;
        let (commands, controls) = mpsc::channel(8);
        let (publisher, states) = watch::channel(MonitorState::default());
        let refresh = Arc::new(Notify::new());
        tokio::spawn(worker(config, controls, publisher, refresh.clone()));
        Ok(Self {
            commands,
            states,
            refresh,
        })
    }
    pub fn subscribe(&self) -> watch::Receiver<MonitorState> {
        self.states.clone()
    }
    pub async fn refresh(&self) -> Result<(), UsageError> {
        if self.states.borrow().status == Status::Stopped {
            return Err(UsageError::Protocol);
        }
        self.refresh.notify_one();
        Ok(())
    }

    pub async fn reconfigure(&self, config: MonitorConfig) -> Result<(), UsageError> {
        config.validate()?;
        let (tx, rx) = oneshot::channel();
        self.commands
            .send(Control::Configure(config, tx))
            .await
            .map_err(|_| UsageError::Protocol)?;
        rx.await.map_err(|_| UsageError::Protocol)
    }
    pub async fn shutdown(&self) {
        let (tx, rx) = oneshot::channel();
        if self.commands.send(Control::Stop(tx)).await.is_ok() {
            let _ = rx.await;
        }
    }
}
fn failure(state: &mut MonitorState, error: UsageError, publisher: &watch::Sender<MonitorState>) {
    state.status = match error {
        UsageError::LoginRequired => Status::LoginRequired,
        UsageError::UnsupportedAuth => Status::UnsupportedAuth,
        _ => Status::Error,
    };
    state.stale = state.snapshot.is_some();
    state.message = Some(error.to_string());
    publisher.send_replace(state.clone());
}
async fn poll(
    client: &mut Client,
    state: &mut MonitorState,
    identity: &mut Option<String>,
    limit_account: &mut Option<String>,
    publisher: &watch::Sender<MonitorState>,
    fetch_tokens: bool,
) -> Result<UsageSnapshot, UsageError> {
    let response = client
        .request("account/read", json!({"refreshToken":false}))
        .await?;
    // This account response supersedes earlier startup/account notifications.
    // Changes received during the subsequent limits read still invalidate it.
    client.drain_notifications();
    let account = &response["account"];
    // Keep identity only in memory. Never send email or authentication material to the UI.
    let key = if account.is_null() {
        None
    } else {
        Some(format!(
            "{}:{}",
            account["type"].as_str().unwrap_or("unknown"),
            account["email"]
                .as_str()
                .or(account["accountId"].as_str())
                .unwrap_or("")
        ))
    };
    if *identity != key {
        state.snapshot = None;
        state.stale = false;
        state.status = Status::Connecting;
        state.message = Some("계정 변경 확인: 사용량 다시 조회 중".into());
        *identity = key;
        *limit_account = None;
        publisher.send_replace(state.clone());
    }
    match account["type"].as_str() {
        None => {
            state.snapshot = None;
            return Err(UsageError::LoginRequired);
        }
        Some("apiKey" | "amazonBedrock" | "bedrockApiKey") => {
            state.snapshot = None;
            return Err(UsageError::UnsupportedAuth);
        }
        _ => {}
    }
    let response = client
        .request(
            "account/rateLimits/read",
            json!({"excludeResetCreditDetails":false}),
        )
        .await?;
    let buckets = parse_limits(response.clone())?;
    if let Some(id) = response["accountId"].as_str() {
        if limit_account.as_deref() != Some(id) {
            state.snapshot = None;
            *limit_account = Some(id.to_owned());
        }
    }
    // The read response is authoritative. Notifications received before it are superseded.
    let auth_changed = client
        .drain_notifications()
        .iter()
        .any(|n| n["method"] == "account/updated");
    if auth_changed {
        *identity = None;
        state.snapshot = None;
        return Err(UsageError::Protocol);
    }
    let fetched_at = now();
    let mut token_summary = state
        .snapshot
        .as_ref()
        .and_then(|s| s.token_summary.clone());
    if fetch_tokens || state.snapshot.is_none() {
        if let Ok(Ok(result)) = tokio::time::timeout(
            Duration::from_secs(5),
            client.request("account/usage/read", json!({})),
        )
        .await
        {
            token_summary = serde_json::from_value(result["summary"].clone()).ok();
        }
    }
    if client.has_auth_change() {
        *identity = None;
        state.snapshot = None;
        return Err(UsageError::Protocol);
    }
    Ok(UsageSnapshot {
        buckets,
        token_summary,
        fetched_at,
        ordinary_usage_allowed: response["ordinaryUsageAllowed"].as_bool(),
        remaining_reset_count: parse_reset_count(&response),
        reset_credit_expirations: parse_reset_expirations(&response),
    })
}
enum Event {
    Command(Option<Control>),
    Connected(Result<Box<Client>, UsageError>),
    Polled(Result<UsageSnapshot, UsageError>),
    Notification(Result<Value, UsageError>),
    Tick,
}
async fn worker(
    mut config: MonitorConfig,
    mut controls: mpsc::Receiver<Control>,
    publisher: watch::Sender<MonitorState>,
    refresh: Arc<Notify>,
) {
    let mut state = MonitorState::default();
    let mut client: Option<Client> = None;
    let mut identity = None;
    let mut limit_account = None;
    let mut next = Instant::now();
    let mut backoff = 2u64;
    let mut polls = 0u64;
    let mut needs_poll = true;
    loop {
        let event = if client.is_none() && Instant::now() < next {
            tokio::select! { biased;
                cmd = controls.recv() => Event::Command(cmd),
                _ = refresh.notified() => Event::Tick,
                _ = tokio::time::sleep_until(next) => Event::Tick,
            }
        } else if client.is_none() {
            tokio::select! { biased;
                cmd = controls.recv() => Event::Command(cmd),
                result = Client::connect(&config.codex_path, Duration::from_secs(15)) => Event::Connected(result.map(Box::new)),
            }
        } else if needs_poll {
            let connection = client.as_mut().expect("checked connection");
            tokio::select! { biased;
                cmd = controls.recv() => Event::Command(cmd),
                result = poll(connection, &mut state, &mut identity, &mut limit_account, &publisher, polls.is_multiple_of(10)) => Event::Polled(result),
            }
        } else {
            let connection = client.as_mut().expect("checked connection");
            tokio::select! { biased;
                cmd = controls.recv() => Event::Command(cmd),
                _ = refresh.notified() => Event::Tick,
                _ = tokio::time::sleep_until(next) => Event::Tick,
                notification = connection.next_notification() => Event::Notification(notification),
            }
        };
        match event {
            Event::Command(Some(Control::Stop(ack))) => {
                if let Some(mut c) = client.take() {
                    c.close().await;
                }
                state.status = Status::Stopped;
                state.stale = state.snapshot.is_some();
                state.message = Some("종료됨".into());
                publisher.send_replace(state);
                let _ = ack.send(());
                return;
            }
            Event::Command(None) => {
                if let Some(mut c) = client.take() {
                    c.close().await;
                }
                return;
            }
            Event::Command(Some(Control::Configure(updated, ack))) => {
                if let Some(mut c) = client.take() {
                    c.close().await;
                }
                config = updated;
                state = MonitorState::default();
                identity = None;
                limit_account = None;
                backoff = 2;
                next = Instant::now();
                needs_poll = true;
                publisher.send_replace(state.clone());
                let _ = ack.send(());
            }
            Event::Connected(Ok(c)) => {
                client = Some(*c);
                needs_poll = true;
            }
            Event::Polled(Ok(snapshot)) => {
                // One pending refresh permit is fulfilled by this successful in-flight read.
                tokio::select! { biased;
                    _ = refresh.notified() => {},
                    _ = std::future::ready(()) => {},
                }
                state = MonitorState {
                    status: Status::Ready,
                    snapshot: Some(snapshot),
                    stale: false,
                    message: None,
                };
                publisher.send_replace(state.clone());
                polls += 1;
                backoff = 2;
                needs_poll = false;
                next = Instant::now() + Duration::from_secs(config.interval_secs);
            }
            Event::Connected(Err(error))
            | Event::Polled(Err(error))
            | Event::Notification(Err(error)) => {
                failure(&mut state, error, &publisher);
                if let Some(mut c) = client.take() {
                    c.close().await;
                }
                next = Instant::now() + Duration::from_secs(backoff);
                backoff = (backoff * 2).min(60);
                needs_poll = true;
            }
            Event::Notification(Ok(value)) if value["method"] == "account/updated" => {
                identity = None;
                limit_account = None;
                state.snapshot = None;
                state.stale = false;
                state.status = Status::Connecting;
                state.message = Some("계정 상태 변경: 다시 조회 중".into());
                publisher.send_replace(state.clone());
                needs_poll = true;
            }
            Event::Notification(Ok(value)) if value["method"] == "account/rateLimits/updated" => {
                if let Ok(updated) = parse_limits(value["params"].clone()) {
                    if let Some(snapshot) = &mut state.snapshot {
                        for bucket in updated {
                            if let Some(existing) =
                                snapshot.buckets.iter_mut().find(|b| b.id == bucket.id)
                            {
                                *existing = bucket;
                            } else {
                                snapshot.buckets.push(bucket);
                            }
                        }
                        snapshot.fetched_at = now();
                        publisher.send_replace(state.clone());
                    } else {
                        needs_poll = true;
                    }
                } else {
                    needs_poll = true;
                }
            }
            Event::Notification(Ok(_)) => {}
            Event::Tick => {
                needs_poll = true;
                next = Instant::now();
            }
        }
    }
}

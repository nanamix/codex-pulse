use crate::UsageError;
use serde_json::{json, Value};
use std::{collections::VecDeque, path::Path, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, Command},
    sync::mpsc,
    task::JoinHandle,
    time::{timeout, Instant},
};

pub struct Client {
    child: Child,
    stdin: ChildStdin,
    incoming: mpsc::Receiver<Result<Value, UsageError>>,
    reader: JoinHandle<()>,
    stderr: JoinHandle<()>,
    id: u64,
    deadline: Duration,
    notifications: VecDeque<Value>,
}
async fn frame<R: AsyncBufRead + Unpin>(reader: &mut R) -> Result<Option<Vec<u8>>, UsageError> {
    let mut line = Vec::new();
    loop {
        let data = reader.fill_buf().await.map_err(|_| UsageError::Io)?;
        if data.is_empty() {
            return if line.is_empty() {
                Ok(None)
            } else {
                Err(UsageError::Protocol)
            };
        }
        let end = data.iter().position(|b| *b == b'\n');
        let count = end.map_or(data.len(), |n| n + 1);
        if line.len() + count > 2 * 1024 * 1024 {
            return Err(UsageError::Protocol);
        }
        line.extend_from_slice(&data[..count]);
        reader.consume(count);
        if end.is_some() {
            return Ok(Some(line));
        }
    }
}
impl Client {
    pub async fn connect(path: &Path, deadline: Duration) -> Result<Self, UsageError> {
        let mut command = Command::new(path);
        #[cfg(windows)]
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
        let mut child = command
            .args(["app-server", "--listen", "stdio://"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| UsageError::Io)?;
        let stdin = child.stdin.take().ok_or(UsageError::Io)?;
        let stdout = child.stdout.take().ok_or(UsageError::Io)?;
        let mut errors = child.stderr.take().ok_or(UsageError::Io)?;
        // Never persist raw stderr: Codex may include account details in it.
        let stderr = tokio::spawn(async move {
            let _ = tokio::io::copy(&mut errors, &mut tokio::io::sink()).await;
        });
        let (tx, incoming) = mpsc::channel(128);
        let reader = tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            loop {
                let result = match frame(&mut reader).await {
                    Ok(Some(line)) => {
                        serde_json::from_slice(&line).map_err(|_| UsageError::Protocol)
                    }
                    _ => Err(UsageError::Protocol),
                };
                let terminal = result.is_err();
                if tx.send(result).await.is_err() || terminal {
                    break;
                }
            }
        });
        let mut client = Self {
            child,
            stdin,
            incoming,
            reader,
            stderr,
            id: 0,
            deadline,
            notifications: VecDeque::new(),
        };
        let initialized = client.request("initialize", json!({"clientInfo":{"name":"codex_usage_monitor","title":"Codex Pulse","version":env!("CARGO_PKG_VERSION")}})).await;
        if let Err(error) = initialized {
            client.close().await;
            return Err(error);
        }
        client.write(&json!({"method":"initialized"})).await?;
        Ok(client)
    }
    async fn write(&mut self, value: &Value) -> Result<(), UsageError> {
        let mut bytes = serde_json::to_vec(value).map_err(|_| UsageError::Protocol)?;
        bytes.push(b'\n');
        timeout(self.deadline, async {
            self.stdin.write_all(&bytes).await?;
            self.stdin.flush().await
        })
        .await
        .map_err(|_| UsageError::Timeout)?
        .map_err(|_| UsageError::Io)
    }
    async fn handle(&mut self, value: Value) -> Result<Option<Value>, UsageError> {
        if value.get("method").is_some() && value.get("id").is_some() {
            self.write(&json!({"id":value["id"], "error":{"code":-32601,"message":"Usage monitor does not handle server requests"}})).await?;
            return Ok(None);
        }
        if value.get("method").is_some() {
            return Ok(Some(value));
        }
        Ok(None)
    }
    pub async fn request(&mut self, method: &str, params: Value) -> Result<Value, UsageError> {
        self.id += 1;
        let id = self.id;
        self.write(&json!({"id":id,"method":method,"params":params}))
            .await?;
        let expires = Instant::now() + self.deadline;
        loop {
            let value = tokio::time::timeout_at(expires, self.incoming.recv())
                .await
                .map_err(|_| UsageError::Timeout)?
                .ok_or(UsageError::Protocol)??;
            if value.get("method").is_none() && value["id"].as_u64() == Some(id) {
                if let Some(error) = value.get("error") {
                    return Err(UsageError::Rpc(error["code"].as_i64().unwrap_or(-32000)));
                }
                return value.get("result").cloned().ok_or(UsageError::Protocol);
            }
            if let Some(notification) = self.handle(value).await? {
                if self.notifications.len() >= 128 {
                    self.notifications.pop_front();
                }
                self.notifications.push_back(notification);
            }
        }
    }
    pub(crate) fn has_auth_change(&self) -> bool {
        self.notifications
            .iter()
            .any(|n| n["method"] == "account/updated")
    }
    pub fn drain_notifications(&mut self) -> Vec<Value> {
        self.notifications.drain(..).collect()
    }
    pub async fn next_notification(&mut self) -> Result<Value, UsageError> {
        if let Some(value) = self.notifications.pop_front() {
            return Ok(value);
        }
        loop {
            let value = self.incoming.recv().await.ok_or(UsageError::Protocol)??;
            if let Some(value) = self.handle(value).await? {
                return Ok(value);
            }
        }
    }
    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }
    pub async fn close(&mut self) {
        let _ = self.child.start_kill();
        let _ = self.child.wait().await;
        self.reader.abort();
        self.stderr.abort();
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        self.reader.abort();
        self.stderr.abort();
        let _ = self.child.start_kill();
    }
}

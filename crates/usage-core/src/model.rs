use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fmt,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaWindow {
    pub used_percent: f64,
    pub window_duration_mins: Option<u64>,
    pub resets_at: Option<i64>,
}
impl QuotaWindow {
    pub fn remaining_percent(&self) -> f64 {
        (100.0 - self.used_percent).clamp(0.0, 100.0)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitBucket {
    pub id: String,
    pub name: Option<String>,
    pub primary: Option<QuotaWindow>,
    pub secondary: Option<QuotaWindow>,
    pub plan_type: Option<String>,
    pub credits: Option<Credits>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Credits {
    pub has_credits: Option<bool>,
    pub unlimited: Option<bool>,
    pub balance: Option<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenSummary {
    pub lifetime_tokens: Option<i64>,
    pub peak_daily_tokens: Option<i64>,
    pub current_streak_days: Option<i64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSnapshot {
    pub buckets: Vec<LimitBucket>,
    pub token_summary: Option<TokenSummary>,
    pub fetched_at: u64,
    pub ordinary_usage_allowed: Option<bool>,
    pub remaining_reset_count: Option<u64>,
    pub reset_credit_expirations: Option<Vec<Option<i64>>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Connecting,
    Ready,
    LoginRequired,
    UnsupportedAuth,
    Error,
    Stopped,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorState {
    pub status: Status,
    pub snapshot: Option<UsageSnapshot>,
    pub stale: bool,
    pub message: Option<String>,
}
impl Default for MonitorState {
    fn default() -> Self {
        Self {
            status: Status::Connecting,
            snapshot: None,
            stale: false,
            message: Some("Codex에 연결 중".into()),
        }
    }
}
#[derive(Debug, Clone)]
pub enum UsageError {
    Io,
    Protocol,
    Timeout,
    Rpc(i64),
    LoginRequired,
    UnsupportedAuth,
    InvalidData,
    InvalidConfig,
}
impl fmt::Display for UsageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Io => "Codex 실행 또는 연결 실패: 실행 파일 경로를 확인하세요",
            Self::Protocol => "Codex 연결이 종료되었거나 응답 형식이 올바르지 않습니다",
            Self::Timeout => "Codex 응답 제한시간을 초과했습니다",
            Self::Rpc(_) => "Codex 조회 요청 실패: 로그인과 네트워크 상태를 확인하세요",
            Self::LoginRequired => "터미널에서 codex login으로 로그인하세요",
            Self::UnsupportedAuth => "이 인증 방식은 ChatGPT 사용 한도를 제공하지 않습니다",
            Self::InvalidData => "서버가 유효한 사용량 정보를 제공하지 않았습니다",
            Self::InvalidConfig => "조회 주기는 5–3600초이며 Codex 경로가 필요합니다",
        })
    }
}
impl std::error::Error for UsageError {}
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
pub fn parse_reset_count(value: &Value) -> Option<u64> {
    value["rateLimitResetCredits"]["availableCount"].as_u64()
}
pub fn parse_reset_expirations(value: &Value) -> Option<Vec<Option<i64>>> {
    value["rateLimitResetCredits"]["credits"]
        .as_array()
        .map(|credits| {
            credits
                .iter()
                .filter(|credit| credit["status"] == "available")
                .filter_map(|credit| match credit.get("expiresAt") {
                    Some(Value::Null) => Some(None),
                    Some(value) => value.as_i64().map(Some),
                    None => None,
                })
                .collect()
        })
}
fn window(value: &Value) -> Result<Option<QuotaWindow>, UsageError> {
    if value.is_null() {
        return Ok(None);
    }
    let w: QuotaWindow =
        serde_json::from_value(value.clone()).map_err(|_| UsageError::InvalidData)?;
    if !w.used_percent.is_finite() || !(0.0..=100.0).contains(&w.used_percent) {
        return Err(UsageError::InvalidData);
    }
    Ok(Some(w))
}
fn bucket(id: &str, value: &Value) -> Result<LimitBucket, UsageError> {
    if !value.is_object() {
        return Err(UsageError::InvalidData);
    }
    Ok(LimitBucket {
        id: id.to_owned(),
        name: value["limitName"].as_str().map(str::to_owned),
        primary: window(&value["primary"])?,
        secondary: window(&value["secondary"])?,
        plan_type: value["planType"].as_str().map(str::to_owned),
        credits: if value["credits"].is_object() {
            serde_json::from_value(value["credits"].clone()).ok()
        } else {
            None
        },
    })
}
pub fn parse_limits(value: Value) -> Result<Vec<LimitBucket>, UsageError> {
    if let Some(map) = value["rateLimitsByLimitId"]
        .as_object()
        .filter(|m| !m.is_empty())
    {
        let mut result: Vec<_> = map
            .iter()
            .map(|(id, v)| bucket(id, v))
            .collect::<Result<_, _>>()?;
        result.sort_by(|a, b| a.id.cmp(&b.id));
        return Ok(result);
    }
    let v = &value["rateLimits"];
    bucket(v["limitId"].as_str().unwrap_or("codex"), v).map(|b| vec![b])
}

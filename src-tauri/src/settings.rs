use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use usage_core::{discover_codex, MonitorConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub show_window: bool,
    pub show_tray: bool,
    pub show_console: bool,
    pub show_touchbar: bool,
    pub always_on_top: bool,
    pub selected_limit_id: Option<String>,
    pub interval_secs: u64,
    pub codex_path: String,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            show_window: true,
            show_tray: true,
            show_console: false,
            show_touchbar: false,
            always_on_top: false,
            selected_limit_id: None,
            interval_secs: 30,
            codex_path: discover_codex().to_string_lossy().into_owned(),
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if !self.show_window && !self.show_tray {
            return Err("다시 열 수 있도록 상태창 또는 메뉴바를 켜 주세요".into());
        }
        self.monitor_config().validate().map_err(|e| e.to_string())
    }
    pub fn monitor_config(&self) -> MonitorConfig {
        MonitorConfig {
            codex_path: PathBuf::from(&self.codex_path),
            interval_secs: self.interval_secs,
        }
    }
}
pub fn load(path: &Path) -> (Settings, Option<String>) {
    match std::fs::read(path) {
        Ok(bytes) => match serde_json::from_slice::<Settings>(&bytes) {
            Ok(s) if s.validate().is_ok() => (s, None),
            _ => (
                Settings::default(),
                Some("저장된 설정이 올바르지 않아 기본 설정을 사용합니다".into()),
            ),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Settings::default(), None),
        Err(_) => (
            Settings::default(),
            Some("설정 파일을 읽을 수 없어 기본 설정을 사용합니다".into()),
        ),
    }
}
pub fn save(path: &Path, settings: &Settings) -> Result<(), String> {
    settings.validate()?;
    let parent = path.parent().ok_or("설정 디렉터리 없음")?;
    std::fs::create_dir_all(parent).map_err(|_| "설정 디렉터리 생성 실패")?;
    let bytes = serde_json::to_vec_pretty(settings).map_err(|_| "설정 직렬화 실패")?;
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, bytes).map_err(|_| "설정 파일 저장 실패")?;
    std::fs::rename(&temporary, path).map_err(|_| "설정 파일 교체 실패".to_owned())
}

use codex_usage_desktop::settings::Settings;
#[test]
fn default_settings_have_accessible_ui_and_thirty_second_interval() {
    let s = Settings::default();
    assert!(s.show_window && s.show_tray);
    assert_eq!(s.interval_secs, 30);
}
#[test]
fn rejects_hidden_app_and_invalid_interval() {
    let mut s = Settings {
        show_window: false,
        show_tray: false,
        show_touchbar: true,
        ..Settings::default()
    };
    assert!(s.validate().is_err());
    s.show_window = true;
    s.interval_secs = 0;
    assert!(s.validate().is_err());
}
#[test]
fn settings_round_trip_and_corruption_is_reported() {
    let dir = std::env::temp_dir().join(format!("codex-settings-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("settings.json");
    let s = Settings {
        show_console: true,
        ..Settings::default()
    };
    codex_usage_desktop::settings::save(&path, &s).unwrap();
    codex_usage_desktop::settings::save(&path, &s).unwrap(); // Existing destination must be replaceable on every OS.
    let (loaded, warning) = codex_usage_desktop::settings::load(&path);
    assert!(loaded.show_console && warning.is_none());
    std::fs::write(&path, "broken").unwrap();
    let (loaded, warning) = codex_usage_desktop::settings::load(&path);
    assert_eq!(loaded.interval_secs, 30);
    assert!(warning.is_some());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn rename_failure_returns_a_string_error_and_preserves_destination() {
    let dir = std::env::temp_dir().join(format!("codex-settings-rename-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let destination = dir.join("settings.json");
    std::fs::create_dir_all(&destination).unwrap();
    let error: String =
        codex_usage_desktop::settings::save(&destination, &Settings::default()).unwrap_err();
    assert_eq!(error, "설정 파일 교체 실패");
    assert!(destination.is_dir());
    std::fs::remove_dir_all(dir).unwrap();
}

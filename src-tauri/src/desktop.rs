use crate::{settings::Settings, touchbar, AppState};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};
use usage_core::MonitorState;

pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "상태창 열기", true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, "refresh", "지금 새로고침", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "종료", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &refresh, &quit])?;
    TrayIconBuilder::with_id("usage")
        .icon(
            app.default_window_icon()
                .expect("configured app icon")
                .clone(),
        )
        .icon_as_template(false)
        .title("Pulse ···")
        .tooltip("Codex 사용량 조회 중")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            "refresh" => {
                let m = app.state::<AppState>().monitor.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = m.refresh().await;
                });
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}
pub fn label(state: &MonitorState, settings: &Settings, name: &str) -> String {
    let bucket = state.snapshot.as_ref().and_then(|s| {
        s.buckets
            .iter()
            .find(|b| Some(&b.id) == settings.selected_limit_id.as_ref())
            .or_else(|| s.buckets.first())
    });
    let window = bucket.and_then(|b| b.primary.as_ref().or(b.secondary.as_ref()));
    match window {
        Some(w) => format!(
            "{name} {:.0}%{}",
            w.remaining_percent(),
            if state.stale { " ⚠" } else { "" }
        ),
        None => format!("{name} —"),
    }
}
pub fn update(app: &AppHandle, state: &MonitorState, settings: &Settings) {
    let text = label(state, settings, "Pulse");
    if let Some(tray) = app.tray_by_id("usage") {
        #[cfg(target_os = "macos")]
        let _ = tray.set_title(Some(text.as_str()));
        let tooltip = format!(
            "Codex {text} · {}",
            if state.stale {
                "이전 조회 데이터"
            } else {
                "상태창에서 자세히 보기"
            }
        );
        let _ = tray.set_tooltip(Some(tooltip.as_str()));
    }
    if let Some(window) = app.get_webview_window("main") {
        let text = label(state, settings, "Codex 5xLite");
        let _ = touchbar::update(&window, settings.show_touchbar, &text);
    }
}
pub fn apply(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    if let Some(tray) = app.tray_by_id("usage") {
        tray.set_visible(settings.show_tray)
            .map_err(|e| e.to_string())?;
    }
    if let Some(window) = app.get_webview_window("main") {
        window
            .set_always_on_top(settings.always_on_top)
            .map_err(|e| e.to_string())?;
        if settings.show_window {
            window.show()
        } else {
            window.hide()
        }
        .map_err(|e| e.to_string())?;
        touchbar::update(&window, settings.show_touchbar, "Codex 5xLite —")?;
    }
    Ok(())
}

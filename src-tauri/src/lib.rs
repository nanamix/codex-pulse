mod desktop;
mod exit_gate;
pub mod settings;
mod touchbar;
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::{Emitter, Manager};
use usage_core::{MonitorHandle, MonitorState};

pub struct AppState {
    monitor: MonitorHandle,
    settings: Mutex<settings::Settings>,
    settings_path: PathBuf,
    startup_warning: Option<String>,
    saving: tokio::sync::Mutex<()>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopState {
    usage: MonitorState,
    settings: settings::Settings,
    startup_warning: Option<String>,
    platform: &'static str,
}
fn snapshot(state: &AppState) -> DesktopState {
    DesktopState {
        usage: state.monitor.subscribe().borrow().clone(),
        settings: state.settings.lock().expect("settings mutex").clone(),
        startup_warning: state.startup_warning.clone(),
        platform: std::env::consts::OS,
    }
}
#[tauri::command]
fn get_state(state: tauri::State<'_, AppState>) -> DesktopState {
    snapshot(&state)
}
#[tauri::command]
fn get_settings(state: tauri::State<'_, AppState>) -> settings::Settings {
    state.settings.lock().expect("settings mutex").clone()
}
#[tauri::command]
async fn refresh_usage(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.monitor.refresh().await.map_err(|e| e.to_string())
}
#[tauri::command]
async fn save_settings(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    settings: settings::Settings,
) -> Result<settings::Settings, String> {
    settings.validate()?;
    let _guard = state.saving.lock().await;
    let previous = state.settings.lock().expect("settings mutex").clone();
    settings::save(&state.settings_path, &settings)?;
    if let Err(error) = desktop::apply(&app, &settings) {
        let _ = desktop::apply(&app, &previous);
        let _ = settings::save(&state.settings_path, &previous);
        return Err(error);
    }
    if previous.codex_path != settings.codex_path
        || previous.interval_secs != settings.interval_secs
    {
        if let Err(error) = state.monitor.reconfigure(settings.monitor_config()).await {
            let _ = desktop::apply(&app, &previous);
            let _ = settings::save(&state.settings_path, &previous);
            return Err(error.to_string());
        }
    }
    *state.settings.lock().expect("settings mutex") = settings.clone();
    let current = snapshot(&state);
    desktop::update(&app, &current.usage, &settings);
    let _ = app.emit("usage-state", current);
    Ok(settings)
}
pub fn run() {
    let app = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_state,
            get_settings,
            refresh_usage,
            save_settings
        ])
        .setup(|app| {
            let path = app.path().app_data_dir()?.join("settings.json");
            let (settings, warning) = settings::load(&path);
            let monitor = {
                let config = settings.monitor_config();
                tauri::async_runtime::block_on(async { MonitorHandle::start(config) })?
            };
            let mut states = monitor.subscribe();
            app.manage(AppState {
                monitor,
                settings: Mutex::new(settings.clone()),
                settings_path: path,
                startup_warning: warning,
                saving: tokio::sync::Mutex::new(()),
            });
            desktop::create_tray(app.handle())?;
            desktop::apply(app.handle(), &settings).map_err(std::io::Error::other)?;
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    let usage = states.borrow_and_update().clone();
                    let app_state = handle.state::<AppState>();
                    let settings = app_state.settings.lock().expect("settings mutex").clone();
                    desktop::update(&handle, &usage, &settings);
                    let _ = handle.emit(
                        "usage-state",
                        DesktopState {
                            usage,
                            settings,
                            startup_warning: app_state.startup_warning.clone(),
                            platform: std::env::consts::OS,
                        },
                    );
                    if states.changed().await.is_err() {
                        break;
                    }
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if window
                    .state::<AppState>()
                    .settings
                    .lock()
                    .expect("settings mutex")
                    .show_tray
                {
                    api.prevent_close();
                    let _ = window.hide();
                } else {
                    window.app_handle().exit(0);
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("Tauri 앱 시작 실패");
    let exiting = Arc::new(exit_gate::ExitGate::default());
    app.run(move |app, event| match event {
        tauri::RunEvent::ExitRequested { api, .. } if !exiting.can_exit() => {
            api.prevent_exit();
            if exiting.request_cleanup() {
                let gate = exiting.clone();
                let monitor = app.state::<AppState>().monitor.clone();
                let handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    monitor.shutdown().await;
                    gate.finish();
                    handle.exit(0);
                });
            }
        }
        #[cfg(target_os = "macos")]
        tauri::RunEvent::Reopen { .. } => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        _ => {}
    });
}

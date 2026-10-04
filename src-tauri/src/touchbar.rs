#[cfg(target_os = "macos")]
use std::ffi::{c_char, c_void, CString};
#[cfg(target_os = "macos")]
extern "C" {
    fn codex_touchbar_update(window: *mut c_void, enabled: bool, text: *const c_char);
}

pub fn update(window: &tauri::WebviewWindow, enabled: bool, label: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let pointer = window.ns_window().map_err(|e| e.to_string())? as usize;
        let text = CString::new(label.replace('\0', "")).map_err(|e| e.to_string())?;
        window
            .run_on_main_thread(move || {
                // Tauri keeps the main window alive until application exit. AppKit calls occur on its main thread.
                unsafe {
                    codex_touchbar_update(pointer as *mut c_void, enabled, text.as_ptr());
                }
            })
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, enabled, label);
    }
    Ok(())
}

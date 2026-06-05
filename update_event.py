with open('tauri-client/src-tauri/src/window_lifecycle.rs', 'r', encoding='utf-8') as f:
    code = f.read()

handle_event = '''
pub fn handle_window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    let label = window.label();
    if label == "screenshot" {
        match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                let _ = window.set_always_on_top(false);
                if let Some(win) = window.as_webview_window() {
                    robust_hide_window(&win);
                }
                crate::CAPTURING.store(false, std::sync::atomic::Ordering::SeqCst);
                api.prevent_close();
            }
            tauri::WindowEvent::Destroyed => {
                crate::CAPTURING.store(false, std::sync::atomic::Ordering::SeqCst);
            }
            _ => {}
        }
    } else if label.starts_with("screenshot_") {
        if let tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed = event {
            crate::CAPTURING.store(false, std::sync::atomic::Ordering::SeqCst);
        }
    } else if label == "recording_border" || label.starts_with("recording_border_") {
        if let tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed = event {
            let _ = window.set_always_on_top(false);
        }
    } else if label == "recording_notice" || label.starts_with("recording_control") {
        match &event {
            tauri::WindowEvent::CloseRequested { .. } => {
                println!("[window-trace] source=window-event action=event label={} event=CloseRequested", label);
                let app_handle = window.app_handle().clone();
                hide_all_app_windows(&app_handle, label);
            }
            tauri::WindowEvent::Destroyed => {
                println!("[window-trace] source=window-event action=destroyed label={} event=Destroyed", label);
                crate::CAPTURING.store(false, std::sync::atomic::Ordering::SeqCst);
                let app_handle = window.app_handle().clone();
                hide_all_app_windows(&app_handle, label);
                let app_handle2 = app_handle.clone();
                let label_owned = label.to_string();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    hide_all_app_windows(&app_handle2, &label_owned);
                    dump_all_windows_state_internal(&app_handle2, format!("recording_control-Destroyed-after-100ms({})", label_owned));
                });
            }
            _ => {}
        }
    } else if label == "main" {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            if let Some(win) = window.as_webview_window() {
                robust_hide_window(&win);
            }
            api.prevent_close();
        }
    }
}
'''
if 'handle_window_event' not in code:
    code += '\n' + handle_event

with open('tauri-client/src-tauri/src/window_lifecycle.rs', 'w', encoding='utf-8') as f:
    f.write(code)

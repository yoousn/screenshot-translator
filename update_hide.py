import re

with open('tauri-client/src-tauri/src/window_lifecycle.rs', 'r', encoding='utf-8') as f:
    lifecycle = f.read()

robust_hide = '''pub fn robust_hide_window<W: tauri::Runtime>(window: &tauri::WebviewWindow<W>) {
    let _ = window.hide();
    #[cfg(target_os = "windows")]
    if let Ok(hwnd) = window.hwnd() {
        unsafe {
            win32::ShowWindow(hwnd.0 as isize, 0); // SW_HIDE
        }
    }
}
'''

# Add robust_hide
lifecycle += '\n' + robust_hide

# Replace window.hide() with robust_hide_window(window) in hide_all_app_windows
old_hide_all = '''pub fn hide_all_app_windows(app: &tauri::AppHandle, trigger: &str) {
    for (lbl, win) in app.webview_windows() {
        if lbl == "main" || lbl == "screenshot" || lbl.starts_with("screenshot_") {
            let is_visible = win.is_visible().unwrap_or(false);
            if is_visible {
                println!("[window-trace] source=hide_all_app_windows action=hide label={} trigger={}", lbl, trigger);
                let _ = win.hide();
            }
        }
    }
}'''

new_hide_all = '''pub fn hide_all_app_windows(app: &tauri::AppHandle, trigger: &str) {
    for (lbl, win) in app.webview_windows() {
        if lbl == "main" || lbl == "screenshot" || lbl.starts_with("screenshot_") {
            println!("[window-trace] source=hide_all_app_windows action=hide label={} trigger={}", lbl, trigger);
            robust_hide_window(&win);
        }
    }
}'''
lifecycle = lifecycle.replace(old_hide_all, new_hide_all)

# Update hide_main_window
lifecycle = lifecycle.replace('let _ = main.hide();', 'robust_hide_window(&main);')

# Update close_screenshot_windows
lifecycle = lifecycle.replace('let _ = window.hide();', 'robust_hide_window(&window);')

with open('tauri-client/src-tauri/src/window_lifecycle.rs', 'w', encoding='utf-8') as f:
    f.write(lifecycle)

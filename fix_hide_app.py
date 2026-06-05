import re

with open('tauri-client/src-tauri/src/window_lifecycle.rs', 'r', encoding='utf-8') as f:
    text = f.read()

text = text.replace('let _ = win.hide();', 'robust_hide_window(&win);')

with open('tauri-client/src-tauri/src/window_lifecycle.rs', 'w', encoding='utf-8') as f:
    f.write(text)

print("Fixed hide_all_app_windows to use robust_hide_window")

import re

with open('tauri-client/src-tauri/src/screenshot_commands.rs', 'r', encoding='utf-8') as f:
    text = f.read()

text = text.replace('let _ = screenshot_win.hide();', 'crate::window_lifecycle::robust_hide_window(&screenshot_win);')

with open('tauri-client/src-tauri/src/screenshot_commands.rs', 'w', encoding='utf-8') as f:
    f.write(text)

print("Fixed screenshot_commands.rs")

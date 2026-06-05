import re

with open('tauri-client/src-tauri/src/lib.rs', 'r', encoding='utf-8') as f:
    lib = f.read()

# Replace the closure content of on_window_event
match = re.search(r'\.on_window_event\(\|window, event\| \{.*?\n\s+\}\)\s*\.run', lib, re.DOTALL)
if match:
    new_event = '.on_window_event(|window, event| { crate::window_lifecycle::handle_window_event(window, event); })\n        .run'
    lib = lib[:match.start()] + new_event + lib[match.end():]
    with open('tauri-client/src-tauri/src/lib.rs', 'w', encoding='utf-8') as f:
        f.write(lib)
else:
    print("Could not find on_window_event block!")

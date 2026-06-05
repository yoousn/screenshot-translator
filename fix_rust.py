import os
import re

os.chdir('tauri-client/src-tauri/src')

def make_fields_pub(filename, struct_names):
    if not os.path.exists(filename): return
    with open(filename, 'r', encoding='utf-8') as f:
        content = f.read()
    
    for struct_name in struct_names:
        # Match the struct block
        pattern = r'(pub\s+struct\s+' + struct_name + r'\s*\{)(.*?)(\})'
        def repl(m):
            body = m.group(2)
            # make fields pub
            body = re.sub(r'(?m)^(\s*)([a-zA-Z0-9_]+)\s*:', r'\1pub \2:', body)
            return m.group(1) + body + m.group(3)
        content = re.sub(pattern, repl, content, flags=re.DOTALL)
        
        # also for tuple structs
        pattern2 = r'(pub\s+struct\s+' + struct_name + r'\s*\()(.*?)(\)\s*;)'
        def repl2(m):
            body = m.group(2)
            # make fields pub
            body = re.sub(r'([a-zA-Z0-9_<>:,\s]+)', r'pub \1', body)
            return m.group(1) + body + m.group(3)
        content = re.sub(pattern2, repl2, content)

    with open(filename, 'w', encoding='utf-8') as f:
        f.write(content)

make_fields_pub('rapid_ocr_commands.rs', ['RapidOcrWorkerProcess', 'RapidOcrCommandSpec'])
make_fields_pub('window_targets.rs', ['RecordingWindowListContext', 'WindowSearchContext'])
make_fields_pub('hotkeys.rs', ['AppShortcutStatus'])

def deduplicate_functions(filename):
    if not os.path.exists(filename): return
    with open(filename, 'r', encoding='utf-8') as f:
        lines = f.readlines()
        
    items = []
    seen = set()
    out_lines = []
    
    i = 0
    buffer_attrs = []
    while i < len(lines):
        line = lines[i]
        stripped = line.strip()
        if stripped.startswith('#[') or stripped.startswith('///'):
            buffer_attrs.append(line)
            i+=1
            continue
            
        match = re.match(r'^(?:pub\s+)?(?:async\s+)?(?:fn|struct|enum|impl|const|static)\s+([a-zA-Z0-9_]+)', line)
        if match:
            name = match.group(1)
            # find end
            if ';' in line and not '{' in line:
                curr = i
            else:
                brace_count = 0
                has_started = False
                curr = i
                while curr < len(lines):
                    cl = re.sub(r'r#*".*?"#*', '', lines[curr])
                    cl = re.sub(r'".*?(?<!\\)"', '', cl)
                    cl = re.sub(r"'.*?'", '', cl)
                    cl = re.sub(r'//.*', '', cl)
                    brace_count += cl.count('{') - cl.count('}')
                    if '{' in cl: has_started = True
                    if has_started and brace_count == 0: break
                    curr += 1
            
            item_lines = buffer_attrs + lines[i:curr+1]
            if name not in seen:
                seen.add(name)
                out_lines.extend(item_lines)
            
            buffer_attrs = []
            i = curr + 1
        else:
            out_lines.extend(buffer_attrs)
            buffer_attrs = []
            out_lines.append(line)
            i+=1
            
    with open(filename, 'w', encoding='utf-8') as f:
        f.write(''.join(out_lines))

deduplicate_functions('lib.rs')
deduplicate_functions('window_lifecycle.rs')
deduplicate_functions('rapid_ocr_commands.rs')

# Make constants pub in lib.rs so others can use them
with open('lib.rs', 'r', encoding='utf-8') as f:
    content = f.read()
content = content.replace('static CAPTURING', 'pub static CAPTURING')
content = content.replace('static LAST_CAPTURE_SHORTCUT_MS', 'pub static LAST_CAPTURE_SHORTCUT_MS')
content = content.replace('const DWMWA_EXTENDED_FRAME_BOUNDS', 'pub const DWMWA_EXTENDED_FRAME_BOUNDS')
content = content.replace('const DWMWA_TRANSITIONS_FORCEDISABLED', 'pub const DWMWA_TRANSITIONS_FORCEDISABLED')
content = content.replace('const SW_SHOW', 'pub const SW_SHOW')
content = content.replace('const HWND_TOPMOST', 'pub const HWND_TOPMOST')
content = content.replace('const SWP_NOMOVE', 'pub const SWP_NOMOVE')
content = content.replace('const SWP_NOSIZE', 'pub const SWP_NOSIZE')
content = content.replace('const SWP_SHOWWINDOW', 'pub const SWP_SHOWWINDOW')

content = content.replace('fn enum_windows_dump_callback', 'pub fn enum_windows_dump_callback')
content = content.replace('fn cleanup_finished_recording_process', 'pub fn cleanup_finished_recording_process')
content = content.replace('fn enum_windows_for_cursor', 'pub fn enum_windows_for_cursor')
content = content.replace('fn enum_child_windows_for_cursor', 'pub fn enum_child_windows_for_cursor')
content = content.replace('fn enum_recording_windows', 'pub fn enum_recording_windows')
content = content.replace('fn hide_recording_overlay_internal', 'pub fn hide_recording_overlay_internal')

with open('lib.rs', 'w', encoding='utf-8') as f:
    f.write(content)

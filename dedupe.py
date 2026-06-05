import os
import re

os.chdir('tauri-client/src-tauri/src')

def fix_file(filename):
    if not os.path.exists(filename): return
    with open(filename, 'r', encoding='utf-8') as f:
        content = f.read()
    content = content.replace('pub pub std::sync::Mutex<Result<()pub pub , String>>', 'pub std::sync::Mutex<Result<(), String>>')
    content = content.replace('pub pub ', 'pub ')
    with open(filename, 'w', encoding='utf-8') as f:
        f.write(content)

fix_file('hotkeys.rs')
fix_file('lib.rs')
fix_file('window_targets.rs')

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

deduplicate_functions('window_targets.rs')
deduplicate_functions('window_lifecycle.rs')
deduplicate_functions('lib.rs')
deduplicate_functions('ffmpeg_dependency.rs')
deduplicate_functions('screenshot_commands.rs')
deduplicate_functions('rapid_ocr_commands.rs')
deduplicate_functions('recording_process.rs')

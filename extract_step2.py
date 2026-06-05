import os, re
os.chdir('tauri-client/src-tauri/src')

MAPPING = {
    "config_store": ["get_config", "save_config", "config_value_string", "config_value_bool", "is_autostart_enabled", "set_autostart_enabled"],
    "history_commands": ["get_history", "get_history_info", "add_history", "clear_history", "choose_history_dir", "HistoryRecord", "history_path_from_config", "history_limits_from_config"],
    "file_commands": ["open_path_in_file_manager", "copy_file_to_clipboard"]
}

for mod_name, mapped_items in MAPPING.items():
    with open('lib.rs', 'r', encoding='utf-8') as f:
        lines = f.readlines()

    out_lib = []
    extracted = []
    i = 0
    buffer_attrs = []

    while i < len(lines):
        line = lines[i]
        stripped = line.strip()
        
        if stripped.startswith('#[') or stripped.startswith('///'):
            buffer_attrs.append(line)
            i+=1
            continue
            
        match = re.match(r'^(?:pub\s+)?(?:async\s+)?(?:fn|struct|enum|const|static)\s+([a-zA-Z0-9_]+)', line)
        if match and match.group(1) in mapped_items:
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
            
            # Ensure it is pub
            decl_idx = len(buffer_attrs)
            if not item_lines[decl_idx].startswith('pub '):
                item_lines[decl_idx] = 'pub ' + item_lines[decl_idx]
                
            extracted.extend(item_lines)
            extracted.append('\n')
            
            buffer_attrs = []
            i = curr + 1
        else:
            out_lib.extend(buffer_attrs)
            buffer_attrs = []
            out_lib.append(line)
            i += 1

    # Write new module
    with open(f'{mod_name}.rs', 'w', encoding='utf-8') as f:
        f.write('use crate::*;\nuse std::path::{Path, PathBuf};\nuse std::fs;\n\n')
        f.write(''.join(extracted))

    # Update lib.rs
    lib_str = ''.join(out_lib)
    lib_str = lib_str.replace('mod text_source;', f'pub mod {mod_name};\npub use {mod_name}::*;\n\nmod text_source;')
    with open('lib.rs', 'w', encoding='utf-8') as f:
        f.write(lib_str)
        
print("Extracted config_store, history_commands, file_commands")

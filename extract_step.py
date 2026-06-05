import os, re
os.chdir('tauri-client/src-tauri/src')

MAPPED = ['app_data_dir', 'repo_root_from_manifest', 'ensure_writable_dir', 'sanitize_tag', 'cleanup_temp_files']

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
    if match and match.group(1) in MAPPED:
        name = match.group(1)
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

# Write app_paths.rs
with open('app_paths.rs', 'w', encoding='utf-8') as f:
    f.write('use crate::*;\nuse std::path::{Path, PathBuf};\nuse std::fs;\n\n')
    f.write(''.join(extracted))

# Update lib.rs
lib_str = ''.join(out_lib)
lib_str = lib_str.replace('mod text_source;', 'pub mod app_paths;\npub use app_paths::*;\n\nmod text_source;')
with open('lib.rs', 'w', encoding='utf-8') as f:
    f.write(lib_str)

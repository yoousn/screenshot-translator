import sys
import re

def main():
    if len(sys.argv) < 4:
        print("Usage: python extract.py <source.rs> <target.rs> <comma_separated_funcs>")
        sys.exit(1)
        
    source_file = sys.argv[1]
    target_file = sys.argv[2]
    funcs_to_extract = sys.argv[3].split(',')
    
    with open(source_file, 'r', encoding='utf-8') as f:
        lines = f.readlines()
        
    # Get all use statements
    use_statements = []
    for line in lines:
        if line.startswith('use ') or line.startswith('pub use '):
            use_statements.append(line)
            
    # Extract
    extracted_lines = []
    remaining_lines = []
    
    i = 0
    while i < len(lines):
        line = lines[i]
        
        # Match item
        # We need to handle #[tauri::command] which precedes the fn
        is_command = False
        start_idx = i
        
        if line.strip() == '#[tauri::command]':
            is_command = True
            i += 1
            while i < len(lines) and (lines[i].strip().startswith('#[') or lines[i].strip() == ''):
                i += 1
            if i < len(lines):
                line_to_check = lines[i]
            else:
                line_to_check = ""
        else:
            line_to_check = line
            
        match = re.match(r'^(?:pub\s+)?(?:async\s+)?(?:fn|struct|enum|impl|const|static)\s+([a-zA-Z0-9_]+)', line_to_check)
        if match:
            name = match.group(1)
            
            # Find end
            if ';' in lines[i] and not '{' in lines[i]:
                # Single line or simple statement
                curr = i
            else:
                brace_count = 0
                has_started = False
                curr = i
                while curr < len(lines):
                    brace_count += lines[curr].count('{') - lines[curr].count('}')
                    if '{' in lines[curr]:
                        has_started = True
                    if has_started and brace_count == 0:
                        break
                    curr += 1
            
            item_lines = lines[start_idx:curr+1]
            if not item_lines[0].startswith('pub ') and not is_command:
                if item_lines[0].startswith('async ') or item_lines[0].startswith('fn ') or item_lines[0].startswith('struct ') or item_lines[0].startswith('enum ') or item_lines[0].startswith('const ') or item_lines[0].startswith('static '):
                    item_lines[0] = 'pub ' + item_lines[0]
            if name in funcs_to_extract:
                extracted_lines.extend(item_lines)
                extracted_lines.append("\n")
                # Ensure the item is pub if it's a fn or struct or enum
                # We can manually add pub later or just regex it:
                # Actually, wait, replacing `fn ` with `pub fn ` might be needed for cross-module usage.
                # Let's not auto-replace pub here, we'll do it manually if needed.
            else:
                remaining_lines.extend(item_lines)
            
            i = curr + 1
        else:
            # Not an item we care about extracting, keep it in source
            if start_idx == i: # no tauri command
                remaining_lines.append(line)
                i += 1
            else: # had tauri command but didn't match
                remaining_lines.extend(lines[start_idx:i+1])
                i += 1

    # Write target file
    target_content = ""
    # Add imports
    for u in use_statements:
        target_content += u
    target_content += "\n"
    target_content += "".join(extracted_lines)
    
    with open(target_file, 'w', encoding='utf-8') as f:
        f.write(target_content)
        
    # Write source file back
    with open(source_file, 'w', encoding='utf-8') as f:
        f.write("".join(remaining_lines))
        
    print(f"Extracted {len(extracted_lines)} lines for {funcs_to_extract} to {target_file}")

if __name__ == '__main__':
    main()

import os
import re

MAPPING = {
    "app_paths": ["app_data_dir", "repo_root_from_manifest", "ensure_writable_dir", "sanitize_tag", "cleanup_temp_files"],
    "config_store": ["get_config", "save_config", "config_value_string", "config_value_bool", "is_autostart_enabled", "set_autostart_enabled"],
    "hotkeys": ["DEFAULT_SCREENSHOT_HOTKEY", "TRANSLATE_HOTKEY_LABEL", "RECORDING_HOTKEY_LABEL", "parse_hotkey", "normalize_key_code", "read_configured_hotkeys", "register_global_shortcuts", "re_register_shortcut", "get_shortcut_status", "AppShortcutStatus", "accept_capture_shortcut_press", "now_epoch_millis"],
    "screenshot_commands": ["start_screenshot", "start_screenshot_impl", "cancel_screenshot", "get_screenshot_image", "capture_region", "capture_live_region", "quick_fullscreen_capture", "get_fullscreen_image", "get_screenshot_pointer_state", "copy_image_to_clipboard", "scroll_mouse_at", "SCREENSHOT_IMAGE", "save_image_to_file", "force_close_screenshots"],
    "window_lifecycle": ["hide_main_window", "show_main_window_safely", "set_window_capture_excluded", "set_webview_capture_excluded", "set_hwnd_capture_excluded", "activate_webview_window", "disable_windows_transition", "close_screenshot_windows", "force_close_recording_controls", "close_recording_windows_safely", "dump_all_windows_state", "dump_all_windows_state_internal", "window_class_name", "enum_windows_dump_callback", "NativeWindowDumpContext", "hide_all_app_windows", "get_main_window"],
    "ffmpeg_dependency": ["find_ffmpeg_executable", "ffmpeg_candidates", "hidden_ffmpeg_command", "get_recording_info", "ffmpeg_input_formats", "collect_ffmpeg_audio_devices", "parse_quoted_audio_devices", "ffmpeg_supports_input_format", "get_ffmpeg_release_info", "download_ffmpeg_release", "choose_ffmpeg_executable", "choose_recording_output_dir", "default_ffmpeg_install_dir", "extract_ffmpeg_exe_from_zip", "emit_ffmpeg_progress", "ffmpeg_stderr_excerpt", "run_ffmpeg_merge", "shell_escape_powershell_single"],
    "recording_process": ["RECORDING_PROCESS", "RecordingOptions", "start_recording", "stop_recording", "cancel_recording_process", "stop_recording_internal", "concat_recording_segments", "cleanup_recording_files", "recording_temp_dir", "build_recording_args", "default_recording_output_dir", "timestamped_recording_file_name", "unique_recording_output_path", "recording_output_path", "resolution_scale_filter", "push_recording_audio_input", "escape_concat_path", "is_recording_temp_file", "get_recording_process", "get_default_recording_output_dir"],
    "history_commands": ["get_history", "get_history_info", "add_history", "clear_history", "choose_history_dir", "HistoryRecord", "history_path_from_config", "history_limits_from_config"],
    "file_commands": ["open_path_in_file_manager", "copy_file_to_clipboard"],
    "window_targets": ["get_recording_targets", "get_window_rects", "get_cursor_position", "current_screen_origin", "hwnd_rect", "push_rect_candidate", "WindowSearchContext", "excluded_app_hwnds", "top_level_windows_at_cursor", "child_windows_at_cursor", "window_title", "process_path_for_hwnd", "exe_name_from_path"],
    "rapid_ocr_commands": ["RapidOcrWorkerProcess", "RAPID_OCR_WORKER", "run_local_ocr", "prewarm_local_ocr_models", "run_rapidocr_sync", "run_local_ocr_sync", "rapid_ocr_worker_state", "get_rapid_ocr_worker_status", "stop_rapid_ocr_worker", "stop_rapid_ocr_worker_internal", "start_rapid_ocr_worker_sync", "run_rapidocr_worker_ocr", "with_rapid_ocr_worker", "rapid_ocr_worker_request_value", "spawn_rapid_ocr_worker_process", "run_rapidocr_json", "resolve_rapidocr_command", "rapid_ocr_runner_candidates", "push_rapid_ocr_runner_candidates_from_base", "write_rapidocr_temp_image", "rapid_ocr_missing_model_files", "rapid_ocr_required_model_files", "rapid_ocr_model_root", "rapid_ocr_model_root_candidates", "push_rapid_ocr_model_candidates_from_base", "push_unique_path", "rapid_ocr_worker_enabled", "rapid_ocr_mode", "rapid_ocr_model_version", "OcrBlock", "RapidOcrRunnerOutput", "RapidOcrCommandSpec", "RapidOcrWorkerEnvelope", "get_rapid_ocr_status", "run_rapid_ocr_self_test", "restart_rapid_ocr_worker"]
}

def clean_for_braces(line):
    line = re.sub(r'r#*".*?"#*', '', line)
    line = re.sub(r'".*?(?<!\\)"', '', line)
    line = re.sub(r"'.*?'", '', line)
    line = re.sub(r'//.*', '', line)
    return line

def parse_items(filepath):
    if not os.path.exists(filepath): return [], []
    with open(filepath, 'r', encoding='utf-8') as f:
        lines = f.readlines()
    
    use_statements = []
    items = []
    i = 0
    buffer_attrs = []
    
    while i < len(lines):
        line = lines[i]
        stripped = line.strip()
        
        if line.startswith('use ') or line.startswith('pub use '):
            if not stripped.endswith('*;'):
                use_statements.append(line)
            buffer_attrs = []
            i += 1
            continue
            
        if stripped == '#[cfg(test)]':
            buffer_attrs = []
            while i < len(lines) and not lines[i].startswith('mod tests'):
                i += 1
            if i < len(lines) and lines[i].startswith('mod tests'):
                brace_count = 0
                if '{' in clean_for_braces(lines[i]):
                    brace_count += clean_for_braces(lines[i]).count('{') - clean_for_braces(lines[i]).count('}')
                i += 1
                while i < len(lines) and brace_count > 0:
                    cl = clean_for_braces(lines[i])
                    brace_count += cl.count('{') - cl.count('}')
                    i += 1
                continue
            continue

        if stripped.startswith('#[') or stripped.startswith('///'):
            buffer_attrs.append(line)
            i += 1
            continue

        if stripped == '':
            buffer_attrs = []
            i += 1
            continue
            
        match = re.match(r'^(?:pub\s+)?(?:async\s+)?(?:fn|struct|enum|impl|const|static)\s+([a-zA-Z0-9_]+)', line)
        if match:
            name = match.group(1)
            is_command = any('#[tauri::command]' in attr for attr in buffer_attrs)
            
            if ';' in clean_for_braces(line) and not '{' in clean_for_braces(line):
                curr = i
            else:
                brace_count = 0
                has_started = False
                curr = i
                while curr < len(lines):
                    cl = clean_for_braces(lines[curr])
                    brace_count += cl.count('{') - cl.count('}')
                    if '{' in cl: has_started = True
                    if has_started and brace_count == 0: break
                    curr += 1
            
            item_lines = buffer_attrs + lines[i:curr+1]
            
            decl_idx = len(buffer_attrs)
            # Ensure everything is public so that there are no visibility issues
            if not item_lines[decl_idx].startswith('pub ') and not item_lines[decl_idx].startswith('impl'):
                item_lines[decl_idx] = 'pub ' + item_lines[decl_idx]
                
            items.append({
                'name': name,
                'lines': item_lines,
                'is_command': is_command
            })
            buffer_attrs = []
            i = curr + 1
        else:
            buffer_attrs = []
            i += 1
    return items, use_statements

def make_struct_fields_pub(item_lines):
    content = "".join(item_lines)
    # Match struct { ... }
    def repl(m):
        body = m.group(2)
        body = re.sub(r'(?m)^(\s*)([a-zA-Z0-9_]+)\s*:', r'\1pub \2:', body)
        return m.group(1) + body + m.group(3)
    content = re.sub(r'(pub\s+struct\s+[a-zA-Z0-9_]+\s*\{)(.*?)(\})', repl, content, flags=re.DOTALL)
    
    # Also fix tuple structs
    def repl2(m):
        body = m.group(2)
        body = re.sub(r'([a-zA-Z0-9_<>:,\s]+)', lambda x: 'pub ' + x.group(1).strip() if x.group(1).strip() else '', body)
        return m.group(1) + body + m.group(3)
    content = re.sub(r'(pub\s+struct\s+[a-zA-Z0-9_]+\s*\()(.*?)(\)\s*;)', repl2, content)
    
    # Specific fix for AppShortcutStatus
    content = content.replace('pub pub std::sync::Mutex', 'pub std::sync::Mutex')
    
    return [content]

def main():
    os.chdir('tauri-client/src-tauri/src')
    
    all_items = []
    all_uses = set()
    sources = ['lib.rs', 'window_control.rs', 'recording_commands.rs', 'recording_process.rs']
    
    for filename in sources:
        items, uses = parse_items(filename)
        all_items.extend(items)
        for u in uses: all_uses.add(u)

    module_contents = {mod: [] for mod in MAPPING.keys()}
    lib_remaining = []
    seen_names = set()
    
    for item in all_items:
        if item['name'] in seen_names:
            continue
        seen_names.add(item['name'])
        
        item['lines'] = make_struct_fields_pub(item['lines'])
        
        found = False
        for mod, names in MAPPING.items():
            if item['name'] in names:
                module_contents[mod].append("".join(item['lines']) + "\n")
                found = True
                break
                
        if not found:
            lib_remaining.extend(item['lines'])
            lib_remaining.append('\n')

    clean_uses = []
    for u in sorted(list(all_uses)):
        if 'crate::window_control' in u or 'crate::recording_process' in u or 'crate::recording_commands' in u:
            continue
        if 'std::os::windows::process::CommandExt' in u:
            clean_uses.append('#[cfg(windows)]\n' + u)
        else:
            clean_uses.append(u)
    
    uses_block = "".join(clean_uses) + "\n"
    
    for mod, contents in module_contents.items():
        with open(f"{mod}.rs", "w", encoding="utf-8") as f:
            f.write(uses_block)
            f.write("use crate::*;\n")
            f.write("use crate::commands::*;\n")
            f.write("".join(contents))
            
    commands_content = "// Auto-generated commands.rs\n"
    for mod in module_contents.keys():
        commands_content += f"pub use crate::{mod}::*;\n"
    with open("commands.rs", "w", encoding="utf-8") as f:
        f.write(commands_content)

    new_lib = uses_block
    new_lib += "pub mod commands;\n"
    for mod in MAPPING.keys():
        new_lib += f"pub mod {mod};\n"
    new_lib += "pub mod text_source;\n"
    new_lib += "pub mod recording_overlay;\n"
    new_lib += "pub use commands::*;\n"
    new_lib += "pub use recording_overlay::*;\n"
    new_lib += "pub use text_source::*;\n"
    
    new_lib += "".join(lib_remaining)
    
    with open("lib.rs", "w", encoding="utf-8") as f:
        f.write(new_lib)
        
    if os.path.exists("window_control.rs"): os.remove("window_control.rs")
    if os.path.exists("recording_commands.rs"): os.remove("recording_commands.rs")
    if os.path.exists("recording_process.rs"): os.remove("recording_process.rs") # It's overwritten anyway, but let's be sure

    print("Perfect extraction complete.")

if __name__ == '__main__':
    main()

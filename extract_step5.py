import os, re
os.chdir('tauri-client/src-tauri/src')

MAPPING = {
    "ffmpeg_dependency": ["find_ffmpeg_executable", "ffmpeg_candidates", "hidden_ffmpeg_command", "get_recording_info", "ffmpeg_input_formats", "collect_ffmpeg_audio_devices", "parse_quoted_audio_devices", "ffmpeg_supports_input_format", "get_ffmpeg_release_info", "download_ffmpeg_release", "choose_ffmpeg_executable", "choose_recording_output_dir", "default_ffmpeg_install_dir", "extract_ffmpeg_exe_from_zip", "emit_ffmpeg_progress", "ffmpeg_stderr_excerpt", "run_ffmpeg_merge", "shell_escape_powershell_single"],
    "rapid_ocr_commands": ["RapidOcrWorkerProcess", "RAPID_OCR_WORKER", "run_local_ocr", "prewarm_local_ocr_models", "run_rapidocr_sync", "run_local_ocr_sync", "rapid_ocr_worker_state", "get_rapid_ocr_worker_status", "stop_rapid_ocr_worker", "stop_rapid_ocr_worker_internal", "start_rapid_ocr_worker_sync", "run_rapidocr_worker_ocr", "with_rapid_ocr_worker", "rapid_ocr_worker_request_value", "spawn_rapid_ocr_worker_process", "run_rapidocr_json", "resolve_rapidocr_command", "rapid_ocr_runner_candidates", "push_rapid_ocr_runner_candidates_from_base", "write_rapidocr_temp_image", "rapid_ocr_missing_model_files", "rapid_ocr_required_model_files", "rapid_ocr_model_root", "rapid_ocr_model_root_candidates", "push_rapid_ocr_model_candidates_from_base", "push_unique_path", "rapid_ocr_worker_enabled", "rapid_ocr_mode", "rapid_ocr_model_version", "OcrBlock", "RapidOcrRunnerOutput", "RapidOcrCommandSpec", "RapidOcrWorkerEnvelope", "get_rapid_ocr_status", "run_rapid_ocr_self_test", "restart_rapid_ocr_worker"]
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
        
print(f"Extracted {list(MAPPING.keys())}")

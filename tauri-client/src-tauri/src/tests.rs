#[cfg(test)]
mod tests {
    use crate::app_paths::sanitize_tag;
    use crate::diagnostics::{
        build_diagnostic_readiness_by_module, startup_diagnostics_probe_path,
    };
    use crate::hotkeys::parse_hotkey;
    use crate::recording_overlay::{
        recording_color_ref, RECORDING_BORDER_BLUE, RECORDING_BORDER_RED, RECORDING_BORDER_YELLOW,
    };
    use crate::recording_process::device_detector::{
        ffmpeg_supports_input_format, parse_quoted_audio_devices,
    };
    use crate::recording_process::ffmpeg_installer::{
        ffmpeg_asset_name_from_url, ffmpeg_checksum_url_for_download, parse_ffmpeg_sha256_manifest,
    };
    use crate::recording_process::process_manager::{
        build_recording_args, cleanup_recording_files, default_recording_output_dir,
        escape_concat_path, ffmpeg_stderr_excerpt, recording_temp_dir, resolution_scale_filter,
        RecordingOptions,
    };
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Deserialize, Serialize)]
    struct RawOcrBlock {
        text: String,
        score: f64,
        box_coords: Vec<Vec<i32>>,
    }

    #[derive(Debug, Serialize)]
    struct OcrBlock {
        text: String,
        confidence: f64,
        box_coords: Vec<Vec<i32>>,
    }

    #[test]
    fn test_raw_score_mapping() {
        let raw_json =
            r#"{"text": "Test OCR", "score": 0.975, "box_coords": [[0,0],[10,0],[10,5],[0,5]]}"#;
        let raw: RawOcrBlock = serde_json::from_str(raw_json).unwrap();
        let mapped = OcrBlock {
            text: raw.text,
            confidence: raw.score,
            box_coords: raw.box_coords,
        };
        assert_eq!(mapped.confidence, 0.975);
        assert_eq!(mapped.text, "Test OCR");
    }

    #[test]
    fn test_parse_hotkey_keeps_minus_as_main_key() {
        assert!(parse_hotkey("Alt+-").is_ok());
        assert!(parse_hotkey("Ctrl+Shift+-").is_ok());
        assert!(parse_hotkey("Alt+Shift+Plus").is_ok());
        assert!(parse_hotkey("Alt++").is_ok());
    }

    #[test]
    fn test_recording_resolution_filter_defaults_to_1080p() {
        assert_eq!(resolution_scale_filter("480p"), Some("scale=-2:480"));
        assert_eq!(resolution_scale_filter("720p"), Some("scale=-2:720"));
        assert_eq!(resolution_scale_filter("1080p"), Some("scale=-2:1080"));
        assert_eq!(resolution_scale_filter("original"), None);
        assert_eq!(resolution_scale_filter("unexpected"), Some("scale=-2:1080"));
    }

    fn recording_options(audio_mode: &str) -> RecordingOptions {
        RecordingOptions {
            fps: Some(60),
            resolution: Some("1080p".to_string()),
            audio_mode: Some(audio_mode.to_string()),
            mic_device: Some("dshow:Microphone Array".to_string()),
            system_audio_device: Some("wasapi:default".to_string()),
            output_dir: None,
            region_x: None,
            region_y: None,
            region_w: None,
            region_h: None,
        }
    }

    fn output_path() -> &'static std::path::Path {
        std::path::Path::new("recording_test.mp4")
    }

    #[test]
    fn test_recording_args_without_audio_use_default_1080p() {
        let options = RecordingOptions {
            fps: None,
            resolution: None,
            audio_mode: None,
            mic_device: None,
            system_audio_device: None,
            output_dir: None,
            region_x: None,
            region_y: None,
            region_w: None,
            region_h: None,
        };
        let args = build_recording_args(&options, output_path()).unwrap();
        assert!(args.windows(2).any(|pair| pair == ["-framerate", "30"]));
        assert!(args.windows(2).any(|pair| pair == ["-r", "30"]));
        assert!(args.windows(2).any(|pair| pair == ["-vf", "scale=-2:1080"]));
        assert!(args.contains(&"-an".to_string()));
        assert_eq!(args.last().unwrap(), "recording_test.mp4");
    }

    #[test]
    fn test_recording_args_original_resolution_omits_scale_filter() {
        let mut options = recording_options("none");
        options.resolution = Some("original".to_string());
        let args = build_recording_args(&options, output_path()).unwrap();
        assert!(!args.contains(&"-vf".to_string()));
    }

    #[test]
    fn test_recording_args_system_audio_uses_wasapi() {
        let args = build_recording_args(&recording_options("system"), output_path()).unwrap();
        assert!(args.windows(2).any(|pair| pair == ["-f", "wasapi"]));
        assert!(args.windows(2).any(|pair| pair == ["-i", "default"]));
        assert!(args.windows(2).any(|pair| pair == ["-map", "1:a"]));
    }

    #[test]
    fn test_recording_args_microphone_uses_dshow() {
        let args = build_recording_args(&recording_options("mic"), output_path()).unwrap();
        assert!(args.windows(2).any(|pair| pair == ["-f", "dshow"]));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["-i", "audio=Microphone Array"]));
    }

    #[test]
    fn test_recording_args_system_and_microphone_mix_audio() {
        let args = build_recording_args(&recording_options("system_mic"), output_path()).unwrap();
        assert!(args.windows(2).any(|pair| pair
            == [
                "-filter_complex",
                "[1:a][2:a]amix=inputs=2:duration=longest[aout]"
            ]));
        assert!(args.windows(2).any(|pair| pair == ["-map", "[aout]"]));
    }

    #[test]
    fn test_recording_args_reject_missing_or_unknown_audio() {
        let mut missing_mic = recording_options("mic");
        missing_mic.mic_device = Some("  ".to_string());
        assert!(build_recording_args(&missing_mic, output_path())
            .unwrap_err()
            .contains("microphone"));

        let unknown = recording_options("speaker_only");
        assert_eq!(
            build_recording_args(&unknown, output_path()).unwrap_err(),
            "Unknown recording audio mode"
        );
    }

    #[test]
    fn test_audio_device_parser_deduplicates_dshow_devices() {
        let output = r#"
[dshow @ 000]  "Microphone Array" (audio)
[dshow @ 000]  "Stereo Mix" (audio)
[dshow @ 000]  "Microphone Array" (audio)
[dshow @ 000]  "USB Camera" (video)
"#;
        let devices = parse_quoted_audio_devices(output, true, None);
        assert_eq!(
            devices,
            vec!["Microphone Array".to_string(), "Stereo Mix".to_string()]
        );
    }

    #[test]
    fn test_audio_device_parser_prefixes_wasapi_devices() {
        let output = r#"
[wasapi @ 000] "default"
[wasapi @ 000] "Speakers (Realtek Audio)"
"#;
        let devices = parse_quoted_audio_devices(output, false, Some("wasapi:"));
        assert_eq!(
            devices,
            vec![
                "wasapi:default".to_string(),
                "wasapi:Speakers (Realtek Audio)".to_string()
            ]
        );
    }

    #[test]
    fn test_ffmpeg_input_format_detection() {
        let output = r#"
File formats:
 D  dshow           DirectShow capture
 DE gdigrab         GDI API Windows frame grabber
  E mp4             MP4 muxer
"#;
        assert!(ffmpeg_supports_input_format(output, "dshow"));
        assert!(ffmpeg_supports_input_format(output, "gdigrab"));
        assert!(!ffmpeg_supports_input_format(output, "wasapi"));
        assert!(!ffmpeg_supports_input_format(output, "mp4"));
    }

    #[test]
    fn test_sanitize_tag_keeps_release_names_filesystem_safe() {
        assert_eq!(sanitize_tag("v1.2.3"), "v1.2.3");
        assert_eq!(sanitize_tag("release/2026:01 beta"), "release_2026_01_beta");
        assert_eq!(sanitize_tag("***"), "___");
    }

    #[test]
    fn test_ffmpeg_checksum_manifest_matches_asset_name() {
        let manifest = "\
b8bed3238f8bf0e3c3388fb3afafc15ef6265ea82999cbf57639a323c6ee7321  ffmpeg-master-latest-win64-gpl-shared.zip\n\
06cd375d0c2051768a727f8d14c1015afc39d2cca7167949153547144fb3df91  ffmpeg-master-latest-win64-gpl.zip\n";
        assert_eq!(
            parse_ffmpeg_sha256_manifest(manifest, "ffmpeg-master-latest-win64-gpl.zip"),
            Some("06cd375d0c2051768a727f8d14c1015afc39d2cca7167949153547144fb3df91".to_string())
        );
        assert_eq!(parse_ffmpeg_sha256_manifest(manifest, "missing.zip"), None);
    }

    #[test]
    fn test_ffmpeg_release_download_url_maps_to_checksum_manifest() {
        let url = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip";
        assert_eq!(
            ffmpeg_asset_name_from_url(url),
            Some("ffmpeg-master-latest-win64-gpl.zip".to_string())
        );
        assert_eq!(
            ffmpeg_checksum_url_for_download(url),
            Some(
                "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/checksums.sha256"
                    .to_string()
            )
        );
    }

    #[test]
    fn test_recording_overlay_status_color_mapping() {
        assert_eq!(recording_color_ref("ready"), RECORDING_BORDER_BLUE);
        assert_eq!(recording_color_ref("recording"), RECORDING_BORDER_RED);
        assert_eq!(recording_color_ref("paused"), RECORDING_BORDER_YELLOW);
        assert_eq!(recording_color_ref("saved"), RECORDING_BORDER_BLUE);
    }

    #[test]
    fn test_default_recording_output_dir_ends_with_ysn() {
        let dir = default_recording_output_dir();
        assert_eq!(
            dir.file_name().and_then(|value| value.to_str()),
            Some("YSN")
        );
    }

    #[test]
    fn test_cleanup_recording_files_only_deletes_temp_mp4() {
        let temp_dir = recording_temp_dir();
        std::fs::create_dir_all(&temp_dir).unwrap();
        let temp_file = temp_dir.join("unit_test_cleanup_boundary.mp4");
        std::fs::write(&temp_file, b"temp").unwrap();

        let external_dir = std::env::temp_dir().join("ysn_recording_boundary_external");
        std::fs::create_dir_all(&external_dir).unwrap();
        let external_file = external_dir.join("unit_test_external.mp4");
        std::fs::write(&external_file, b"external").unwrap();

        cleanup_recording_files(vec![
            temp_file.to_string_lossy().to_string(),
            external_file.to_string_lossy().to_string(),
        ])
        .unwrap();

        assert!(!temp_file.exists());
        assert!(external_file.exists());

        let _ = std::fs::remove_file(external_file);
        let _ = std::fs::remove_dir(external_dir);
    }

    #[test]
    fn test_escape_concat_path_uses_ffmpeg_file_list_syntax() {
        let path = std::path::Path::new(r"C:\Users\Alice\Videos\Bob's clip.mp4");
        assert_eq!(
            escape_concat_path(path),
            "C:/Users/Alice/Videos/Bob\\'s clip.mp4"
        );
    }

    #[test]
    fn test_ffmpeg_stderr_excerpt_keeps_tail_context() {
        let stderr = (0..20)
            .map(|index| format!("line {}", index))
            .collect::<Vec<_>>()
            .join("\n");
        let excerpt = ffmpeg_stderr_excerpt(stderr.as_bytes());
        assert!(!excerpt.contains("line 0"));
        assert!(excerpt.contains("line 19"));
    }

    #[test]
    fn test_startup_diagnostics_probe_path_is_in_temp_dir() {
        let path = startup_diagnostics_probe_path();
        assert!(path.starts_with(std::env::temp_dir()));
        assert_eq!(
            path.file_name().and_then(|value| value.to_str()),
            Some("startup_status.json")
        );
    }

    #[test]
    fn test_diagnostic_readiness_by_module_keeps_ocr_not_ready() {
        let ocr_runtime = serde_json::json!({
            "ready": false,
            "readinessSteps": [
                { "id": "rapidocr-runner", "ready": true },
                { "id": "rapidocr-probe", "ready": false, "nextAction": "run-ocr-self-test" }
            ]
        });
        let recording = serde_json::json!({ "ffmpegFound": false, "audioDevices": [] });
        let readiness = build_diagnostic_readiness_by_module(&ocr_runtime, &recording);
        assert_eq!(readiness["ocrRuntime"]["ready"].as_bool(), Some(false));
        assert_eq!(readiness["ocrRuntime"]["readySteps"].as_u64(), Some(1));
        assert_eq!(readiness["ocrRuntime"]["totalSteps"].as_u64(), Some(2));
        assert_eq!(
            readiness["ocrRuntime"]["firstBlockedStep"]["id"].as_str(),
            Some("rapidocr-probe")
        );
        assert_eq!(readiness["recording"]["ready"].as_bool(), Some(false));
    }
}

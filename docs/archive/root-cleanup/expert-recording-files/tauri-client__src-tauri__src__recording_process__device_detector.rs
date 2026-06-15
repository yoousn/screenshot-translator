use std::process::Command;
use std::path::Path;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub fn parse_quoted_audio_devices(
    output: &str,
    audio_marker_required: bool,
    prefix: Option<&str>,
) -> Vec<String> {
    let mut devices = Vec::new();
    for line in output.lines() {
        if audio_marker_required && !line.contains("(audio)") {
            continue;
        }
        if let Some(first_quote) = line.find('"') {
            if let Some(second_quote) = line[first_quote + 1..].find('"') {
                let name = line[first_quote + 1..first_quote + 1 + second_quote].trim();
                if !name.is_empty() {
                    let value = match prefix {
                        Some(prefix) => format!("{}{}", prefix, name),
                        None => name.to_string(),
                    };
                    if !devices.contains(&value) {
                        devices.push(value);
                    }
                }
            }
        }
    }
    devices
}

pub fn ffmpeg_supports_input_format(formats_output: &str, format_name: &str) -> bool {
    formats_output.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("D") && trimmed.split_whitespace().nth(1) == Some(format_name)
    })
}

pub fn hidden_ffmpeg_command(ffmpeg_path: &Path) -> Command {
    let mut cmd = Command::new(ffmpeg_path);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

pub fn ffmpeg_input_formats(ffmpeg_path: &Path) -> String {
    hidden_ffmpeg_command(ffmpeg_path)
        .args(["-hide_banner", "-formats"])
        .output()
        .map(|out| {
            format!(
                "{}\n{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            )
        })
        .unwrap_or_default()
}

pub fn collect_ffmpeg_audio_devices(ffmpeg_path: &Path) -> Vec<String> {
    let mut devices = Vec::new();
    let input_formats = ffmpeg_input_formats(ffmpeg_path);
    if let Ok(out) = hidden_ffmpeg_command(ffmpeg_path)
        .args([
            "-hide_banner",
            "-list_devices",
            "true",
            "-f",
            "dshow",
            "-i",
            "dummy",
        ])
        .output()
    {
        let combined = format!(
            "{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        devices.extend(parse_quoted_audio_devices(&combined, true, None));
    }
    if ffmpeg_supports_input_format(&input_formats, "wasapi") {
        if let Ok(out) = hidden_ffmpeg_command(ffmpeg_path)
            .args([
                "-hide_banner",
                "-list_devices",
                "true",
                "-f",
                "wasapi",
                "-i",
                "dummy",
            ])
            .output()
        {
            let combined = format!(
                "{}\n{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            devices.extend(parse_quoted_audio_devices(
                &combined,
                false,
                Some("wasapi:"),
            ));
        }
        if !devices.contains(&"wasapi:default".to_string()) {
            devices.push("wasapi:default".to_string());
        }
    }
    devices
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeWgcOneFrameProbeSmokeRequest {
    pub explicit_opt_in: Option<bool>,
    pub allow_real_wgc_api: Option<bool>,
    pub frame_timeout_ms: Option<u64>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeDxgiSelectedReadbackSmokeRequest {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub explicit_opt_in: Option<bool>,
    pub allow_real_dxgi_api: Option<bool>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeDxgiDefaultVsSelectedAcquireComparisonRequest {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub explicit_opt_in: Option<bool>,
    pub allow_real_dxgi_api: Option<bool>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeCursorNudgeSmokeRequest {
    pub dx: Option<i32>,
    pub dy: Option<i32>,
    pub explicit_opt_in: Option<bool>,
    pub allow_real_cursor_nudge: Option<bool>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeDxgiCursorNudgeDiagnosticRequest {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub dx: Option<i32>,
    pub dy: Option<i32>,
    pub explicit_opt_in: Option<bool>,
    pub allow_real_dxgi_api: Option<bool>,
    pub allow_real_cursor_nudge: Option<bool>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeDxgiFrameInfoProbeRequest {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub explicit_opt_in: Option<bool>,
    pub allow_real_dxgi_api: Option<bool>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeDxgiDesktopUpdatePulseDiagnosticRequest {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub explicit_opt_in: Option<bool>,
    pub allow_real_dxgi_api: Option<bool>,
    pub allow_real_desktop_pulse: Option<bool>,
    pub pulse_size_px: Option<u32>,
    pub pulse_alpha: Option<u8>,
    pub dwell_ms: Option<u64>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeDxgiPulseBeforeAcquireProbeRequest {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub explicit_opt_in: Option<bool>,
    pub allow_real_dxgi_api: Option<bool>,
    pub allow_real_desktop_pulse: Option<bool>,
    pub pulse_size_px: Option<u32>,
    pub pulse_alpha: Option<u8>,
    pub dwell_ms: Option<u64>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeDxgiSelectedOutputBridgeDryRunRequest {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub explicit_opt_in: Option<bool>,
    pub allow_real_dxgi_api: Option<bool>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeDxgiSelectedOutputClipboardAcceptanceRequest {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub explicit_opt_in: Option<bool>,
    pub allow_real_dxgi_api: Option<bool>,
    pub allow_fake_clipboard_sink: Option<bool>,
    pub allow_real_clipboard: Option<bool>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeWgcMonitorTargetDiagnosticRequest {
    pub bounds: Option<NativeDxgiSelectedReadbackSmokeRequest>,
    pub validate: Option<bool>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeWgcMonitorSessionSmokeRequest {
    pub bounds: Option<NativeDxgiSelectedReadbackSmokeRequest>,
    pub explicit_opt_in: Option<bool>,
    pub allow_real_wgc_api: Option<bool>,
    pub frame_timeout_ms: Option<u64>,
    pub include_cursor: Option<bool>,
    pub require_border: Option<bool>,
    pub buffer_count: Option<i32>,
    pub validate_target: Option<bool>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeWgcSelectedOutputClipboardAcceptanceRequest {
    pub bounds: Option<NativeDxgiSelectedReadbackSmokeRequest>,
    pub explicit_opt_in: Option<bool>,
    pub allow_real_wgc_api: Option<bool>,
    pub allow_fake_clipboard_sink: Option<bool>,
    pub allow_real_clipboard: Option<bool>,
    pub frame_timeout_ms: Option<u64>,
    pub include_cursor: Option<bool>,
    pub require_border: Option<bool>,
    pub buffer_count: Option<i32>,
    pub validate_target: Option<bool>,
    pub include_selected_png_base64: Option<bool>,
    pub allow_file_write: Option<bool>,
    pub save_path: Option<String>,
}

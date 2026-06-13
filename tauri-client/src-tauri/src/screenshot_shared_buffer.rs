#[cfg(target_os = "windows")]
use std::sync::mpsc::{channel, RecvTimeoutError};
#[cfg(target_os = "windows")]
use std::time::Duration;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ScreenshotSharedBufferPostResult {
    pub posted: bool,
    pub transfer_type: String,
    pub session_id: String,
    pub bytes: usize,
    pub width: u32,
    pub height: u32,
    pub reason: Option<String>,
}

impl ScreenshotSharedBufferPostResult {
    #[cfg(not(target_os = "windows"))]
    fn unavailable(session_id: String, reason: impl Into<String>) -> Self {
        Self {
            posted: false,
            transfer_type: SCREENSHOT_TRANSFER_TYPE.to_string(),
            session_id,
            bytes: 0,
            width: 0,
            height: 0,
            reason: Some(reason.into()),
        }
    }
}

const SCREENSHOT_TRANSFER_TYPE: &str = "screenshot";
const IMAGE_EXTRA_INFO_BYTES: usize = 8;
#[cfg(target_os = "windows")]
const SHARED_BUFFER_POST_TIMEOUT_MS: u64 = 500;

pub fn build_rgba_shared_buffer_payload(
    frame: &crate::screenshot_native::RgbaFrame,
) -> Result<Vec<u8>, String> {
    let expected = frame
        .expected_byte_len()
        .ok_or_else(|| "RGBA frame dimensions overflow".to_string())?;
    if expected != frame.bytes.len() {
        return Err(format!(
            "RGBA frame byte length mismatch: expected={} actual={}",
            expected,
            frame.bytes.len()
        ));
    }
    let total_len = frame
        .bytes
        .len()
        .checked_add(IMAGE_EXTRA_INFO_BYTES)
        .ok_or_else(|| "SharedBuffer payload length overflow".to_string())?;
    let mut payload = Vec::with_capacity(total_len);
    payload.extend_from_slice(&frame.bytes);
    payload.extend_from_slice(&frame.width.to_le_bytes());
    payload.extend_from_slice(&frame.height.to_le_bytes());
    Ok(payload)
}

#[cfg(target_os = "windows")]
pub fn post_rgba_frame_to_webview(
    webview: tauri::Webview,
    session_id: String,
    frame: &crate::screenshot_native::RgbaFrame,
) -> Result<ScreenshotSharedBufferPostResult, String> {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2Environment12, ICoreWebView2_17, COREWEBVIEW2_SHARED_BUFFER_ACCESS_READ_ONLY,
    };
    use windows_core::{Interface, PCWSTR};

    let payload = build_rgba_shared_buffer_payload(frame)?;
    let payload_len = payload.len();
    let width = frame.width;
    let height = frame.height;
    let transfer_type = SCREENSHOT_TRANSFER_TYPE.to_string();
    let additional_data = serde_json::json!({
        "transfer_type": transfer_type,
        "session_id": session_id,
    })
    .to_string();

    let (sender, receiver) = channel::<Result<(), String>>();
    let sender_for_webview = sender.clone();
    let with_webview_result = webview.with_webview(move |platform_webview| {
        let environment = platform_webview.environment();
        let controller = platform_webview.controller();

        let core_webview = match unsafe { controller.CoreWebView2() } {
            Ok(core_webview) => core_webview,
            Err(error) => {
                let _ = sender_for_webview.send(Err(format!(
                    "failed to get CoreWebView2 from controller: {error:?}"
                )));
                return;
            }
        };

        let environment_12 = match environment.cast::<ICoreWebView2Environment12>() {
            Ok(environment_12) => environment_12,
            Err(error) => {
                let _ = sender_for_webview.send(Err(format!(
                    "WebView2 environment does not support SharedBuffer: {error:?}"
                )));
                return;
            }
        };

        let shared_buffer = match unsafe { environment_12.CreateSharedBuffer(payload_len as u64) } {
            Ok(shared_buffer) => shared_buffer,
            Err(error) => {
                let _ = sender_for_webview.send(Err(format!(
                    "failed to create WebView2 SharedBuffer: {error:?}"
                )));
                return;
            }
        };

        let mut shared_buffer_ptr: *mut u8 = std::ptr::null_mut();
        if let Err(error) = unsafe { shared_buffer.Buffer(&mut shared_buffer_ptr) } {
            let _ = sender_for_webview.send(Err(format!(
                "failed to obtain WebView2 SharedBuffer pointer: {error:?}"
            )));
            return;
        }
        if shared_buffer_ptr.is_null() {
            let _ =
                sender_for_webview.send(Err("WebView2 SharedBuffer pointer was null".to_string()));
            return;
        }

        unsafe {
            std::ptr::copy_nonoverlapping(payload.as_ptr(), shared_buffer_ptr, payload.len());
        }

        let webview_17 = match core_webview.cast::<ICoreWebView2_17>() {
            Ok(webview_17) => webview_17,
            Err(error) => {
                let _ = sender_for_webview.send(Err(format!(
                    "WebView2 runtime does not support PostSharedBufferToScript: {error:?}"
                )));
                return;
            }
        };

        let additional_data_wide: Vec<u16> = additional_data
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let additional_data_pcwstr = PCWSTR::from_raw(additional_data_wide.as_ptr());

        let post_result = unsafe {
            webview_17.PostSharedBufferToScript(
                &shared_buffer,
                COREWEBVIEW2_SHARED_BUFFER_ACCESS_READ_ONLY,
                additional_data_pcwstr,
            )
        };
        match post_result {
            Ok(()) => {
                let _ = sender_for_webview.send(Ok(()));
            }
            Err(error) => {
                let _ = sender_for_webview.send(Err(format!(
                    "failed to post WebView2 SharedBuffer to script: {error:?}"
                )));
            }
        }
    });

    if let Err(error) = with_webview_result {
        return Err(format!("failed to enter WebView2 context: {error}"));
    }

    match receiver.recv_timeout(Duration::from_millis(SHARED_BUFFER_POST_TIMEOUT_MS)) {
        Ok(Ok(())) => Ok(ScreenshotSharedBufferPostResult {
            posted: true,
            transfer_type,
            session_id,
            bytes: payload_len,
            width,
            height,
            reason: None,
        }),
        Ok(Err(error)) => Err(error),
        Err(RecvTimeoutError::Timeout) => Err(format!(
            "timed out after {SHARED_BUFFER_POST_TIMEOUT_MS}ms waiting for WebView2 SharedBuffer post"
        )),
        Err(RecvTimeoutError::Disconnected) => {
            Err("failed to receive WebView2 SharedBuffer post result".to_string())
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn post_rgba_frame_to_webview(
    _webview: tauri::Webview,
    session_id: String,
    _frame: &crate::screenshot_native::RgbaFrame,
) -> Result<ScreenshotSharedBufferPostResult, String> {
    Ok(ScreenshotSharedBufferPostResult::unavailable(
        session_id,
        "WebView2 SharedBuffer is only available on Windows",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_buffer_payload_appends_dimensions_little_endian() {
        let frame = crate::screenshot_native::RgbaFrame::new(2, 1, vec![1, 2, 3, 4, 5, 6, 7, 8])
            .expect("valid frame");

        let payload = build_rgba_shared_buffer_payload(&frame).expect("payload");

        assert_eq!(payload.len(), 16);
        assert_eq!(&payload[0..8], &[1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(&payload[8..12], &2_u32.to_le_bytes());
        assert_eq!(&payload[12..16], &1_u32.to_le_bytes());
    }

    #[test]
    fn shared_buffer_payload_rejects_invalid_byte_count() {
        let frame = crate::screenshot_native::RgbaFrame {
            width: 2,
            height: 1,
            bytes: vec![1, 2, 3, 4],
        };

        assert!(build_rgba_shared_buffer_payload(&frame).is_err());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn shared_buffer_post_wait_is_bounded() {
        assert!((1..=1000).contains(&SHARED_BUFFER_POST_TIMEOUT_MS));
    }
}

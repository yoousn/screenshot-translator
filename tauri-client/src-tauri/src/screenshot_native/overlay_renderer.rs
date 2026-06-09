use std::fmt;

use super::RgbaFrame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayRendererBackend {
    CpuRgbaGdiDib,
}

impl fmt::Display for OverlayRendererBackend {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::CpuRgbaGdiDib => "cpu-rgba-gdi-dib",
        };
        formatter.write_str(name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayRendererPixelFormat {
    Rgba8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayRendererAlphaMode {
    Straight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayRendererOrientation {
    TopDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlayRendererContract {
    pub backend: OverlayRendererBackend,
    pub input_format: OverlayRendererPixelFormat,
    pub alpha_mode: OverlayRendererAlphaMode,
    pub orientation: OverlayRendererOrientation,
    pub requires_tightly_packed_rgba: bool,
    pub uses_window_dc: bool,
    pub owns_message_loop: bool,
}

impl OverlayRendererContract {
    pub const fn cpu_rgba_gdi_dib() -> Self {
        Self {
            backend: OverlayRendererBackend::CpuRgbaGdiDib,
            input_format: OverlayRendererPixelFormat::Rgba8,
            alpha_mode: OverlayRendererAlphaMode::Straight,
            orientation: OverlayRendererOrientation::TopDown,
            requires_tightly_packed_rgba: true,
            uses_window_dc: true,
            owns_message_loop: false,
        }
    }
}

impl Default for OverlayRendererContract {
    fn default() -> Self {
        Self::cpu_rgba_gdi_dib()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlayRenderTarget {
    pub hwnd: isize,
    pub dst_x: i32,
    pub dst_y: i32,
    pub dst_width: u32,
    pub dst_height: u32,
}

impl OverlayRenderTarget {
    pub const fn hwnd(hwnd: isize, width: u32, height: u32) -> Self {
        Self {
            hwnd,
            dst_x: 0,
            dst_y: 0,
            dst_width: width,
            dst_height: height,
        }
    }

    pub const fn with_position(
        hwnd: isize,
        dst_x: i32,
        dst_y: i32,
        dst_width: u32,
        dst_height: u32,
    ) -> Self {
        Self {
            hwnd,
            dst_x,
            dst_y,
            dst_width,
            dst_height,
        }
    }

    pub const fn is_empty(self) -> bool {
        self.hwnd == 0 || self.dst_width == 0 || self.dst_height == 0
    }
}

pub type OverlayRenderResult<T> = Result<T, OverlayRenderError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayRenderError {
    pub kind: OverlayRenderErrorKind,
    pub recoverable: bool,
    pub message: String,
}

impl OverlayRenderError {
    pub fn recoverable(kind: OverlayRenderErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            recoverable: true,
            message: message.into(),
        }
    }

    pub fn fatal(kind: OverlayRenderErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            recoverable: false,
            message: message.into(),
        }
    }
}

impl fmt::Display for OverlayRenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for OverlayRenderError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlayRenderErrorKind {
    EmptyTarget,
    InvalidFrameLength { expected: usize, actual: usize },
    DimensionOverflow,
    UnsupportedPlatform,
    DeviceContextUnavailable,
    GdiBlitFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlayRenderReceipt {
    pub backend: OverlayRendererBackend,
    pub hwnd: isize,
    pub width: u32,
    pub height: u32,
    pub bytes_uploaded: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayRenderDiagnostics {
    pub backend: OverlayRendererBackend,
    pub target: OverlayRenderTarget,
    pub frame_width: u32,
    pub frame_height: u32,
    pub frame_byte_len: usize,
    pub expected_byte_len: Option<usize>,
    pub tightly_packed_rgba: bool,
    pub can_attempt_render: bool,
    pub failure: Option<OverlayRenderError>,
}

impl OverlayRenderDiagnostics {
    pub fn from_request(target: OverlayRenderTarget, frame: &RgbaFrame) -> Self {
        let expected_byte_len = expected_rgba_len(frame.width, frame.height).ok();
        let failure = validate_render_request(target, frame).err();

        Self {
            backend: OverlayRendererBackend::CpuRgbaGdiDib,
            target,
            frame_width: frame.width,
            frame_height: frame.height,
            frame_byte_len: frame.bytes.len(),
            expected_byte_len,
            tightly_packed_rgba: expected_byte_len == Some(frame.bytes.len()),
            can_attempt_render: failure.is_none(),
            failure,
        }
    }
}

pub fn overlay_renderer_contract() -> OverlayRendererContract {
    OverlayRendererContract::cpu_rgba_gdi_dib()
}

pub fn render_rgba_frame_to_overlay(
    target: OverlayRenderTarget,
    frame: &RgbaFrame,
) -> OverlayRenderResult<OverlayRenderReceipt> {
    validate_render_request(target, frame)?;
    render_rgba_frame_to_platform_overlay(target, frame)
}

pub fn diagnose_overlay_render_request(
    target: OverlayRenderTarget,
    frame: &RgbaFrame,
) -> OverlayRenderDiagnostics {
    OverlayRenderDiagnostics::from_request(target, frame)
}

fn validate_render_request(
    target: OverlayRenderTarget,
    frame: &RgbaFrame,
) -> OverlayRenderResult<()> {
    if target.is_empty() {
        return Err(OverlayRenderError::recoverable(
            OverlayRenderErrorKind::EmptyTarget,
            "native overlay render target is empty",
        ));
    }

    let expected = expected_rgba_len(frame.width, frame.height)?;
    if frame.bytes.len() != expected {
        return Err(OverlayRenderError::recoverable(
            OverlayRenderErrorKind::InvalidFrameLength {
                expected,
                actual: frame.bytes.len(),
            },
            format!(
                "native overlay frame length mismatch: expected {expected}, got {}",
                frame.bytes.len()
            ),
        ));
    }

    let _ = checked_i32_dimension(frame.width)?;
    let _ = checked_i32_dimension(frame.height)?;
    let _ = checked_i32_dimension(target.dst_width)?;
    let _ = checked_i32_dimension(target.dst_height)?;
    Ok(())
}

fn expected_rgba_len(width: u32, height: u32) -> OverlayRenderResult<usize> {
    usize::try_from(width)
        .ok()
        .and_then(|width| width.checked_mul(usize::try_from(height).ok()?))
        .and_then(|pixels| pixels.checked_mul(RgbaFrame::BYTES_PER_PIXEL))
        .ok_or_else(|| {
            OverlayRenderError::fatal(
                OverlayRenderErrorKind::DimensionOverflow,
                "native overlay frame dimensions overflow RGBA buffer length",
            )
        })
}

fn checked_i32_dimension(value: u32) -> OverlayRenderResult<i32> {
    i32::try_from(value).map_err(|_| {
        OverlayRenderError::fatal(
            OverlayRenderErrorKind::DimensionOverflow,
            "native overlay render dimension exceeds Win32 i32 range",
        )
    })
}

#[cfg(target_os = "windows")]
fn render_rgba_frame_to_platform_overlay(
    target: OverlayRenderTarget,
    frame: &RgbaFrame,
) -> OverlayRenderResult<OverlayRenderReceipt> {
    let dst_width = checked_i32_dimension(target.dst_width)?;
    let dst_height = checked_i32_dimension(target.dst_height)?;
    let src_width = checked_i32_dimension(frame.width)?;
    let src_height = checked_i32_dimension(frame.height)?;
    let device_context = unsafe { GetDC(target.hwnd) };
    if device_context == 0 {
        return Err(OverlayRenderError::recoverable(
            OverlayRenderErrorKind::DeviceContextUnavailable,
            "GetDC(native overlay) failed",
        ));
    }

    let mut bitmap_info = bitmap_info_for_rgba_frame(frame.width, frame.height)?;
    let result = unsafe {
        StretchDIBits(
            device_context,
            target.dst_x,
            target.dst_y,
            dst_width,
            dst_height,
            0,
            0,
            src_width,
            src_height,
            frame.bytes.as_ptr().cast(),
            &mut bitmap_info,
            DIB_RGB_COLORS,
            SRCCOPY,
        )
    };
    let _ = unsafe { ReleaseDC(target.hwnd, device_context) };

    if result == 0 || result == GDI_ERROR {
        return Err(OverlayRenderError::recoverable(
            OverlayRenderErrorKind::GdiBlitFailed,
            "StretchDIBits(native overlay RGBA frame) failed",
        ));
    }

    Ok(OverlayRenderReceipt {
        backend: OverlayRendererBackend::CpuRgbaGdiDib,
        hwnd: target.hwnd,
        width: target.dst_width,
        height: target.dst_height,
        bytes_uploaded: frame.bytes.len(),
    })
}

#[cfg(not(target_os = "windows"))]
fn render_rgba_frame_to_platform_overlay(
    target: OverlayRenderTarget,
    frame: &RgbaFrame,
) -> OverlayRenderResult<OverlayRenderReceipt> {
    let _ = target;
    let _ = frame;
    Err(OverlayRenderError::fatal(
        OverlayRenderErrorKind::UnsupportedPlatform,
        "native overlay CPU RGBA/GDI/DIB renderer is Windows-only",
    ))
}

#[cfg(target_os = "windows")]
fn bitmap_info_for_rgba_frame(width: u32, height: u32) -> OverlayRenderResult<BitmapInfo> {
    Ok(BitmapInfo {
        header: BitmapInfoHeader {
            bi_size: std::mem::size_of::<BitmapInfoHeader>() as u32,
            bi_width: checked_i32_dimension(width)?,
            bi_height: -checked_i32_dimension(height)?,
            bi_planes: 1,
            bi_bit_count: 32,
            bi_compression: BI_RGB,
            bi_size_image: 0,
            bi_x_pels_per_meter: 0,
            bi_y_pels_per_meter: 0,
            bi_clr_used: 0,
            bi_clr_important: 0,
        },
        colors: [0; 3],
    })
}

#[cfg(target_os = "windows")]
#[repr(C)]
struct BitmapInfoHeader {
    bi_size: u32,
    bi_width: i32,
    bi_height: i32,
    bi_planes: u16,
    bi_bit_count: u16,
    bi_compression: u32,
    bi_size_image: u32,
    bi_x_pels_per_meter: i32,
    bi_y_pels_per_meter: i32,
    bi_clr_used: u32,
    bi_clr_important: u32,
}

#[cfg(target_os = "windows")]
#[repr(C)]
struct BitmapInfo {
    header: BitmapInfoHeader,
    colors: [u32; 3],
}

#[cfg(target_os = "windows")]
const BI_RGB: u32 = 0;
#[cfg(target_os = "windows")]
const DIB_RGB_COLORS: u32 = 0;
#[cfg(target_os = "windows")]
const SRCCOPY: u32 = 0x00CC0020;
#[cfg(target_os = "windows")]
const GDI_ERROR: i32 = -1;

#[cfg(target_os = "windows")]
#[link(name = "user32")]
extern "system" {
    fn GetDC(hWnd: isize) -> isize;
    fn ReleaseDC(hWnd: isize, hDC: isize) -> i32;
}

#[cfg(target_os = "windows")]
#[link(name = "gdi32")]
extern "system" {
    fn StretchDIBits(
        hdc: isize,
        x_dest: i32,
        y_dest: i32,
        dest_width: i32,
        dest_height: i32,
        x_src: i32,
        y_src: i32,
        src_width: i32,
        src_height: i32,
        bits: *const std::ffi::c_void,
        bitmap_info: *const BitmapInfo,
        usage: u32,
        rop: u32,
    ) -> i32;
}

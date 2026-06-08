use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureBackendKind {
    ExistingCpu,
    WindowsGraphicsCapture,
    DesktopDuplication,
}

impl CaptureBackendKind {
    pub const fn contract(self) -> CaptureBackendContract {
        match self {
            Self::ExistingCpu => CaptureBackendContract::cpu(),
            Self::WindowsGraphicsCapture => CaptureBackendContract::wgc(),
            Self::DesktopDuplication => CaptureBackendContract::dxgi(),
        }
    }

    pub const fn is_gpu(self) -> bool {
        matches!(
            self,
            Self::WindowsGraphicsCapture | Self::DesktopDuplication
        )
    }
}

impl fmt::Display for CaptureBackendKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::ExistingCpu => "existing-cpu",
            Self::WindowsGraphicsCapture => "windows-graphics-capture",
            Self::DesktopDuplication => "desktop-duplication",
        };
        formatter.write_str(name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapturePixelFormat {
    Rgba8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureReadbackMode {
    CpuMemory,
    GpuTextureReadback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureBackendContract {
    pub backend: CaptureBackendKind,
    pub output_format: CapturePixelFormat,
    pub readback_mode: CaptureReadbackMode,
    pub supports_region_capture: bool,
    pub supports_negative_origin: bool,
    pub may_return_protected_content: bool,
}

impl CaptureBackendContract {
    pub const fn cpu() -> Self {
        Self {
            backend: CaptureBackendKind::ExistingCpu,
            output_format: CapturePixelFormat::Rgba8,
            readback_mode: CaptureReadbackMode::CpuMemory,
            supports_region_capture: true,
            supports_negative_origin: true,
            may_return_protected_content: false,
        }
    }

    pub const fn wgc() -> Self {
        Self {
            backend: CaptureBackendKind::WindowsGraphicsCapture,
            output_format: CapturePixelFormat::Rgba8,
            readback_mode: CaptureReadbackMode::GpuTextureReadback,
            supports_region_capture: true,
            supports_negative_origin: true,
            may_return_protected_content: true,
        }
    }

    pub const fn dxgi() -> Self {
        Self {
            backend: CaptureBackendKind::DesktopDuplication,
            output_format: CapturePixelFormat::Rgba8,
            readback_mode: CaptureReadbackMode::GpuTextureReadback,
            supports_region_capture: true,
            supports_negative_origin: true,
            may_return_protected_content: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonitorCaptureBounds {
    pub origin_x: i32,
    pub origin_y: i32,
    pub width: u32,
    pub height: u32,
}

impl MonitorCaptureBounds {
    pub const fn new(origin_x: i32, origin_y: i32, width: u32, height: u32) -> Self {
        Self {
            origin_x,
            origin_y,
            width,
            height,
        }
    }

    pub fn from_tuple(value: (i32, i32, u32, u32)) -> Self {
        Self::new(value.0, value.1, value.2, value.3)
    }

    pub fn as_tuple(self) -> (i32, i32, u32, u32) {
        (self.origin_x, self.origin_y, self.width, self.height)
    }

    pub const fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }

    pub fn right(self) -> Option<i32> {
        self.origin_x.checked_add(i32::try_from(self.width).ok()?)
    }

    pub fn bottom(self) -> Option<i32> {
        self.origin_y.checked_add(i32::try_from(self.height).ok()?)
    }

    pub fn area_pixels(self) -> Option<usize> {
        usize::try_from(self.width)
            .ok()?
            .checked_mul(usize::try_from(self.height).ok()?)
    }

    pub fn rgba_byte_len(self) -> Option<usize> {
        self.area_pixels()?.checked_mul(RgbaFrame::BYTES_PER_PIXEL)
    }

    pub fn contains_point(self, x: i32, y: i32) -> bool {
        let Some(right) = self.right() else {
            return false;
        };
        let Some(bottom) = self.bottom() else {
            return false;
        };
        x >= self.origin_x && y >= self.origin_y && x < right && y < bottom
    }

    pub fn intersects(self, other: Self) -> bool {
        let Some(self_right) = self.right() else {
            return false;
        };
        let Some(self_bottom) = self.bottom() else {
            return false;
        };
        let Some(other_right) = other.right() else {
            return false;
        };
        let Some(other_bottom) = other.bottom() else {
            return false;
        };
        self.origin_x < other_right
            && self_right > other.origin_x
            && self.origin_y < other_bottom
            && self_bottom > other.origin_y
    }
}

#[derive(Debug, Clone)]
pub struct RgbaFrame {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl RgbaFrame {
    pub const BYTES_PER_PIXEL: usize = 4;

    pub fn new(width: u32, height: u32, bytes: Vec<u8>) -> CaptureResult<Self> {
        let frame = Self {
            bytes,
            width,
            height,
        };
        frame.validate()?;
        Ok(frame)
    }

    pub fn from_bounds(bounds: MonitorCaptureBounds, bytes: Vec<u8>) -> CaptureResult<Self> {
        Self::new(bounds.width, bounds.height, bytes)
    }

    pub fn bounds_at(&self, origin_x: i32, origin_y: i32) -> MonitorCaptureBounds {
        MonitorCaptureBounds::new(origin_x, origin_y, self.width, self.height)
    }

    pub fn byte_len(&self) -> usize {
        self.bytes.len()
    }

    pub fn expected_byte_len(&self) -> Option<usize> {
        self.bounds_at(0, 0).rgba_byte_len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty() || self.width == 0 || self.height == 0
    }

    pub fn is_tightly_packed_rgba(&self) -> bool {
        self.expected_byte_len() == Some(self.bytes.len())
    }

    pub fn validate(&self) -> CaptureResult<()> {
        let bounds = self.bounds_at(0, 0);
        let expected = bounds
            .rgba_byte_len()
            .ok_or(CaptureError::InvalidBounds(bounds))?;
        if self.bytes.len() != expected {
            return Err(CaptureError::InvalidFrameLength {
                expected,
                actual: self.bytes.len(),
                bounds,
            });
        }
        Ok(())
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureError {
    BackendUnavailable {
        backend: CaptureBackendKind,
        reason: String,
    },
    InvalidBounds(MonitorCaptureBounds),
    InvalidFrameLength {
        expected: usize,
        actual: usize,
        bounds: MonitorCaptureBounds,
    },
    ProtectedContent {
        backend: CaptureBackendKind,
        bounds: MonitorCaptureBounds,
    },
    ReadbackFailed {
        backend: CaptureBackendKind,
        reason: String,
    },
}

impl fmt::Display for CaptureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BackendUnavailable { backend, reason } => {
                write!(formatter, "capture backend {backend} unavailable: {reason}")
            }
            Self::InvalidBounds(bounds) => write!(
                formatter,
                "invalid capture bounds: x={}, y={}, width={}, height={}",
                bounds.origin_x, bounds.origin_y, bounds.width, bounds.height
            ),
            Self::InvalidFrameLength {
                expected,
                actual,
                bounds,
            } => write!(
                formatter,
                "invalid rgba frame length: expected {expected}, got {actual} for {}x{}",
                bounds.width, bounds.height
            ),
            Self::ProtectedContent { backend, bounds } => write!(
                formatter,
                "capture backend {backend} returned protected content for {}x{}",
                bounds.width, bounds.height
            ),
            Self::ReadbackFailed { backend, reason } => {
                write!(
                    formatter,
                    "capture backend {backend} readback failed: {reason}"
                )
            }
        }
    }
}

impl std::error::Error for CaptureError {}

pub type CaptureResult<T> = Result<T, CaptureError>;

pub trait CaptureFrameSource {
    fn backend_kind(&self) -> CaptureBackendKind;

    fn backend_contract(&self) -> CaptureBackendContract {
        self.backend_kind().contract()
    }

    fn capture_bounds(&self) -> Option<MonitorCaptureBounds> {
        None
    }

    fn capture_frame(&mut self, bounds: MonitorCaptureBounds) -> CaptureResult<RgbaFrame>;
}

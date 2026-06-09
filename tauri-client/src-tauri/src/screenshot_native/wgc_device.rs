use std::num::NonZeroU64;

use super::d3d11_frame::{
    D3d11CpuAccessMode, D3d11FrameSource, D3d11RawHandle, D3d11TextureFrame,
    D3d11TextureFrameFormat, D3d11TextureFrameMetadata, D3d11TextureHandle, D3d11TextureUsage,
};
use super::gpu_device::{
    create_d3d11_capture_device, D3d11DeviceCreateOptions, D3d11DeviceDiagnostics,
    D3d11DeviceError, D3d11DeviceHandle,
};
use super::output::SelectedImageContract;

#[cfg(windows)]
use windows::core::Interface;
#[cfg(windows)]
use windows::Graphics::DirectX::Direct3D11::{IDirect3DDevice, IDirect3DSurface};
#[cfg(windows)]
use windows::Win32::Graphics::Direct3D11::{ID3D11Texture2D, D3D11_TEXTURE2D_DESC};
#[cfg(windows)]
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_R8G8B8A8_UNORM,
};
#[cfg(windows)]
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
#[cfg(windows)]
use windows::Win32::System::WinRT::Direct3D11::{
    CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WgcDeviceBridgeError {
    UnsupportedPlatform(&'static str),
    D3d11Device(String),
    WinRtDevice(String),
    SurfaceInterop(String),
    EmptyDimensions { width: u32, height: u32 },
    UnsupportedFormat { format: String },
    MissingTextureHandle,
    InvalidFrameContract { reason: String },
}

impl std::fmt::Display for WgcDeviceBridgeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedPlatform(reason) => formatter.write_str(reason),
            Self::D3d11Device(reason) => write!(formatter, "WGC D3D11 device failed: {reason}"),
            Self::WinRtDevice(reason) => {
                write!(formatter, "WGC WinRT device bridge failed: {reason}")
            }
            Self::SurfaceInterop(reason) => {
                write!(formatter, "WGC surface interop failed: {reason}")
            }
            Self::EmptyDimensions { width, height } => {
                write!(formatter, "empty WGC texture dimensions: {width}x{height}")
            }
            Self::UnsupportedFormat { format } => {
                write!(formatter, "unsupported WGC texture format: {format}")
            }
            Self::MissingTextureHandle => formatter.write_str("WGC texture handle is missing"),
            Self::InvalidFrameContract { reason } => {
                write!(formatter, "invalid WGC texture frame contract: {reason}")
            }
        }
    }
}

impl std::error::Error for WgcDeviceBridgeError {}

impl From<D3d11DeviceError> for WgcDeviceBridgeError {
    fn from(error: D3d11DeviceError) -> Self {
        Self::D3d11Device(error.to_string())
    }
}

pub type WgcDeviceBridgeResult<T> = Result<T, WgcDeviceBridgeError>;

#[derive(Debug)]
pub struct WgcDirect3DDeviceBridge {
    pub d3d11: D3d11DeviceHandle,
    #[cfg(windows)]
    pub direct3d: IDirect3DDevice,
}

impl WgcDirect3DDeviceBridge {
    pub fn diagnostics(&self) -> D3d11DeviceDiagnostics {
        self.d3d11.diagnostics.clone()
    }
}

#[derive(Debug, Clone)]
pub struct WgcDeviceBridgeProbe {
    pub attempted: bool,
    pub ok: bool,
    pub created_d3d11_device: bool,
    pub created_direct3d_device: bool,
    pub diagnostics: Option<D3d11DeviceDiagnostics>,
    pub error: Option<String>,
}

pub fn probe_wgc_direct3d_device_bridge() -> WgcDeviceBridgeProbe {
    match create_wgc_direct3d_device(D3d11DeviceCreateOptions::default()) {
        Ok(bridge) => WgcDeviceBridgeProbe {
            attempted: true,
            ok: true,
            created_d3d11_device: true,
            created_direct3d_device: true,
            diagnostics: Some(bridge.diagnostics()),
            error: None,
        },
        Err(error) => WgcDeviceBridgeProbe {
            attempted: true,
            ok: false,
            created_d3d11_device: false,
            created_direct3d_device: false,
            diagnostics: None,
            error: Some(error.to_string()),
        },
    }
}

#[cfg(not(windows))]
pub fn create_wgc_direct3d_device(
    _options: D3d11DeviceCreateOptions,
) -> WgcDeviceBridgeResult<WgcDirect3DDeviceBridge> {
    Err(WgcDeviceBridgeError::UnsupportedPlatform(
        "WGC Direct3D device bridge requires Windows",
    ))
}

#[cfg(windows)]
pub fn create_wgc_direct3d_device(
    options: D3d11DeviceCreateOptions,
) -> WgcDeviceBridgeResult<WgcDirect3DDeviceBridge> {
    let d3d11 = create_d3d11_capture_device(options)?;
    let dxgi_device = d3d11
        .device
        .cast::<IDXGIDevice>()
        .map_err(|error| WgcDeviceBridgeError::WinRtDevice(error.to_string()))?;
    let inspectable = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device) }
        .map_err(|error| WgcDeviceBridgeError::WinRtDevice(error.to_string()))?;
    let direct3d = inspectable
        .cast::<IDirect3DDevice>()
        .map_err(|error| WgcDeviceBridgeError::WinRtDevice(error.to_string()))?;
    Ok(WgcDirect3DDeviceBridge { d3d11, direct3d })
}

#[cfg(windows)]
pub fn d3d11_texture_from_wgc_surface(
    surface: &IDirect3DSurface,
) -> WgcDeviceBridgeResult<ID3D11Texture2D> {
    let access = surface
        .cast::<IDirect3DDxgiInterfaceAccess>()
        .map_err(|error| WgcDeviceBridgeError::SurfaceInterop(error.to_string()))?;
    unsafe { access.GetInterface::<ID3D11Texture2D>() }
        .map_err(|error| WgcDeviceBridgeError::SurfaceInterop(error.to_string()))
}

#[cfg(windows)]
pub struct WgcAcquiredTextureFrame {
    frame: D3d11TextureFrame,
    _texture: ID3D11Texture2D,
    selected_image: Option<SelectedImageContract>,
}

#[cfg(windows)]
impl WgcAcquiredTextureFrame {
    pub fn frame_contract(&self) -> &D3d11TextureFrame {
        &self.frame
    }

    pub fn into_frame_contract(self) -> D3d11TextureFrame {
        self.frame
    }

    pub fn texture(&self) -> &ID3D11Texture2D {
        &self._texture
    }

    pub fn selected_image(&self) -> Option<&SelectedImageContract> {
        self.selected_image.as_ref()
    }

    pub fn set_selected_image(&mut self, selected_image: SelectedImageContract) {
        self.selected_image = Some(selected_image);
    }
}

#[cfg(windows)]
pub fn describe_wgc_d3d11_texture_2d(
    texture: ID3D11Texture2D,
    frame_id: u64,
) -> WgcDeviceBridgeResult<WgcAcquiredTextureFrame> {
    let mut desc = D3D11_TEXTURE2D_DESC::default();
    unsafe { texture.GetDesc(&mut desc) };
    let format = match desc.Format {
        DXGI_FORMAT_B8G8R8A8_UNORM => D3d11TextureFrameFormat::Bgra8Unorm,
        DXGI_FORMAT_R8G8B8A8_UNORM => D3d11TextureFrameFormat::Rgba8Unorm,
        _ => {
            return Err(WgcDeviceBridgeError::UnsupportedFormat {
                format: format!("{:?}", desc.Format),
            })
        }
    };
    let raw = texture.as_raw() as usize as u64;
    let raw_handle = NonZeroU64::new(raw)
        .map(D3d11RawHandle::new)
        .ok_or(WgcDeviceBridgeError::MissingTextureHandle)?;
    let metadata = D3d11TextureFrameMetadata::new(
        D3d11FrameSource::WindowsGraphicsCapture,
        desc.Width,
        desc.Height,
        format,
    )
    .with_texture(D3d11TextureHandle::new(
        raw_handle,
        D3d11TextureUsage::CaptureTexture,
        D3d11CpuAccessMode::None,
    ))
    .with_frame_id(frame_id);
    let frame = D3d11TextureFrame::new(metadata, None).map_err(|error| {
        WgcDeviceBridgeError::InvalidFrameContract {
            reason: error.to_string(),
        }
    })?;
    Ok(WgcAcquiredTextureFrame {
        frame,
        _texture: texture,
        selected_image: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_windows_probe_reports_attempted_failure() {
        let report = probe_wgc_direct3d_device_bridge();
        assert!(report.attempted);
        if cfg!(windows) {
            assert!(report.ok || report.error.is_some());
        } else {
            assert!(!report.ok);
            assert!(report.error.unwrap_or_default().contains("Windows"));
        }
    }

    #[test]
    fn bridge_error_display_is_stable() {
        let error = WgcDeviceBridgeError::UnsupportedFormat {
            format: "Unknown".to_string(),
        };
        assert!(error.to_string().contains("unsupported WGC texture format"));
    }
}

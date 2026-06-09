#[cfg(windows)]
use std::num::NonZeroU64;

use super::d3d11_frame::{
    D3d11CpuAccessMode, D3d11FrameSource, D3d11RawHandle, D3d11TextureFrame,
    D3d11TextureFrameFormat, D3d11TextureFrameMetadata, D3d11TextureHandle, D3d11TextureUsage,
};

#[cfg(windows)]
use windows::core::Interface;
#[cfg(windows)]
use windows::Win32::Graphics::Direct3D11::{ID3D11Texture2D, D3D11_TEXTURE2D_DESC};
#[cfg(windows)]
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_R8G8B8A8_UNORM,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DxgiTextureFrameError {
    EmptyDimensions { width: u32, height: u32 },
    UnsupportedFormat { format: String },
    MissingTextureHandle,
    InvalidFrameContract { reason: String },
}

impl std::fmt::Display for DxgiTextureFrameError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyDimensions { width, height } => {
                write!(formatter, "empty DXGI texture dimensions: {width}x{height}")
            }
            Self::UnsupportedFormat { format } => {
                write!(formatter, "unsupported DXGI texture format: {format}")
            }
            Self::MissingTextureHandle => formatter.write_str("DXGI texture handle is missing"),
            Self::InvalidFrameContract { reason } => {
                write!(formatter, "invalid DXGI texture frame contract: {reason}")
            }
        }
    }
}

impl std::error::Error for DxgiTextureFrameError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DxgiTextureDescriptor {
    pub width: u32,
    pub height: u32,
    pub format: D3d11TextureFrameFormat,
    pub raw_handle: D3d11RawHandle,
    pub frame_id: u64,
}

impl DxgiTextureDescriptor {
    pub const fn new(
        width: u32,
        height: u32,
        format: D3d11TextureFrameFormat,
        raw_handle: D3d11RawHandle,
        frame_id: u64,
    ) -> Self {
        Self {
            width,
            height,
            format,
            raw_handle,
            frame_id,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DxgiAcquiredTextureFrame {
    frame: D3d11TextureFrame,
    #[cfg(windows)]
    _texture: ID3D11Texture2D,
}

impl DxgiAcquiredTextureFrame {
    pub fn metadata(&self) -> D3d11TextureFrameMetadata {
        self.frame.metadata
    }

    pub fn frame_contract(&self) -> &D3d11TextureFrame {
        &self.frame
    }

    #[cfg(windows)]
    pub fn texture(&self) -> Result<&ID3D11Texture2D, DxgiTextureFrameError> {
        Ok(&self._texture)
    }

    pub fn into_frame_contract(self) -> D3d11TextureFrame {
        self.frame
    }
}

pub fn build_dxgi_texture_frame_contract(
    descriptor: DxgiTextureDescriptor,
) -> Result<D3d11TextureFrame, DxgiTextureFrameError> {
    if descriptor.width == 0 || descriptor.height == 0 {
        return Err(DxgiTextureFrameError::EmptyDimensions {
            width: descriptor.width,
            height: descriptor.height,
        });
    }

    let texture = D3d11TextureHandle::new(
        descriptor.raw_handle,
        D3d11TextureUsage::CaptureTexture,
        D3d11CpuAccessMode::None,
    );
    let metadata = D3d11TextureFrameMetadata::new(
        D3d11FrameSource::DxgiDesktopDuplication,
        descriptor.width,
        descriptor.height,
        descriptor.format,
    )
    .with_texture(texture)
    .with_frame_id(descriptor.frame_id);

    D3d11TextureFrame::new(metadata, None).map_err(|error| {
        DxgiTextureFrameError::InvalidFrameContract {
            reason: error.to_string(),
        }
    })
}

#[cfg(windows)]
pub fn describe_dxgi_d3d11_texture_2d(
    texture: ID3D11Texture2D,
    frame_id: u64,
) -> Result<DxgiAcquiredTextureFrame, DxgiTextureFrameError> {
    let mut desc = D3D11_TEXTURE2D_DESC::default();
    unsafe { texture.GetDesc(&mut desc) };
    let format = match desc.Format {
        DXGI_FORMAT_B8G8R8A8_UNORM => D3d11TextureFrameFormat::Bgra8Unorm,
        DXGI_FORMAT_R8G8B8A8_UNORM => D3d11TextureFrameFormat::Rgba8Unorm,
        _ => {
            return Err(DxgiTextureFrameError::UnsupportedFormat {
                format: format!("{:?}", desc.Format),
            })
        }
    };
    let raw = texture.as_raw() as usize as u64;
    let raw_handle = NonZeroU64::new(raw)
        .map(D3d11RawHandle::new)
        .ok_or(DxgiTextureFrameError::MissingTextureHandle)?;
    let frame = build_dxgi_texture_frame_contract(DxgiTextureDescriptor::new(
        desc.Width,
        desc.Height,
        format,
        raw_handle,
        frame_id,
    ))?;
    Ok(DxgiAcquiredTextureFrame {
        frame,
        _texture: texture,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroU64;

    #[test]
    fn texture_frame_contract_keeps_gpu_texture_without_readback() {
        let descriptor = DxgiTextureDescriptor::new(
            1920,
            1080,
            D3d11TextureFrameFormat::Bgra8Unorm,
            D3d11RawHandle::new(NonZeroU64::new(0x44).expect("non-zero handle")),
            7,
        );

        let frame = build_dxgi_texture_frame_contract(descriptor).expect("valid texture frame");

        assert!(frame.readback_bytes.is_none());
        assert!(frame.clone().requires_gpu_texture());
        assert_eq!(frame.metadata.width, 1920);
        assert_eq!(frame.metadata.height, 1080);
        assert_eq!(frame.metadata.frame_id, 7);
        assert_eq!(
            frame.metadata.texture.expect("texture handle").cpu_access,
            D3d11CpuAccessMode::None
        );
    }

    #[test]
    fn texture_frame_contract_rejects_empty_dimensions() {
        let descriptor = DxgiTextureDescriptor::new(
            0,
            1080,
            D3d11TextureFrameFormat::Bgra8Unorm,
            D3d11RawHandle::new(NonZeroU64::new(0x44).expect("non-zero handle")),
            7,
        );

        assert!(matches!(
            build_dxgi_texture_frame_contract(descriptor),
            Err(DxgiTextureFrameError::EmptyDimensions { .. })
        ));
    }
}

#[cfg(windows)]
use super::d3d11_frame::{
    D3d11FrameContractError, D3d11FrameResult, D3d11FrameSource, D3d11StagingReadbackMetadata,
    D3d11TextureFrame, D3d11TextureFrameFormat, D3d11TextureFrameMetadata,
    D3D11_RGBA_BYTES_PER_PIXEL,
};
#[cfg(windows)]
use super::output::{CropRect, ImageBounds, SelectedImageContract, SelectionRect};
#[cfg(windows)]
use super::selected_image_bridge::{
    build_selected_png_contract_from_source, SelectedImageBridgeError, SelectedImageBridgeResult,
    SelectedRegionReadbackSource,
};

#[cfg(windows)]
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Device, ID3D11DeviceContext, ID3D11Resource, ID3D11Texture2D, D3D11_BOX,
    D3D11_CPU_ACCESS_READ, D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_TEXTURE2D_DESC,
    D3D11_USAGE_STAGING,
};

#[cfg(windows)]
use windows::core::Interface;

#[cfg(windows)]
pub struct DxgiD3d11SelectedRegionReadbackSource<'a> {
    device: &'a ID3D11Device,
    context: &'a ID3D11DeviceContext,
    texture: &'a ID3D11Texture2D,
    bounds: ImageBounds,
}

#[cfg(windows)]
impl<'a> DxgiD3d11SelectedRegionReadbackSource<'a> {
    pub const fn new(
        device: &'a ID3D11Device,
        context: &'a ID3D11DeviceContext,
        texture: &'a ID3D11Texture2D,
        bounds: ImageBounds,
    ) -> Self {
        Self {
            device,
            context,
            texture,
            bounds,
        }
    }
}

#[cfg(windows)]
impl SelectedRegionReadbackSource for DxgiD3d11SelectedRegionReadbackSource<'_> {
    fn bounds(&self) -> ImageBounds {
        self.bounds
    }

    fn validate_readback(&self) -> SelectedImageBridgeResult<()> {
        if self.bounds.is_empty() {
            return Err(SelectedImageBridgeError::InvalidFrame(
                "DXGI selected readback source bounds are empty".into(),
            ));
        }
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        unsafe { self.texture.GetDesc(&mut desc) };
        dxgi_texture_format(desc.Format)
            .map(|_| ())
            .map_err(|error| SelectedImageBridgeError::InvalidFrame(error.to_string()))?;
        if self.bounds.width > desc.Width || self.bounds.height > desc.Height {
            return Err(SelectedImageBridgeError::InvalidFrame(format!(
                "DXGI selected readback bounds {}x{} exceed texture {}x{}",
                self.bounds.width, self.bounds.height, desc.Width, desc.Height
            )));
        }
        Ok(())
    }

    fn read_selected_rgba(&self, crop: CropRect) -> SelectedImageBridgeResult<Vec<u8>> {
        readback_dxgi_d3d11_texture_2d_region_rgba(self.device, self.context, self.texture, crop)
            .map_err(|error| SelectedImageBridgeError::InvalidFrame(error.to_string()))
    }
}

#[cfg(windows)]
pub fn build_selected_png_contract_from_dxgi_texture(
    device: &ID3D11Device,
    context: &ID3D11DeviceContext,
    texture: &ID3D11Texture2D,
    bounds: ImageBounds,
    selection: SelectionRect,
) -> SelectedImageBridgeResult<SelectedImageContract> {
    let source = DxgiD3d11SelectedRegionReadbackSource::new(device, context, texture, bounds);
    build_selected_png_contract_from_source(&source, selection)
}

#[cfg(windows)]
pub fn readback_dxgi_d3d11_texture_2d(
    device: &ID3D11Device,
    context: &ID3D11DeviceContext,
    texture: &ID3D11Texture2D,
    frame_id: u64,
) -> D3d11FrameResult<D3d11TextureFrame> {
    let mut desc = D3D11_TEXTURE2D_DESC::default();
    unsafe { texture.GetDesc(&mut desc) };
    let format = dxgi_texture_format(desc.Format)?;
    desc.Usage = D3D11_USAGE_STAGING;
    desc.BindFlags = 0;
    desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
    desc.MiscFlags = 0;

    let mut staging = None;
    unsafe { device.CreateTexture2D(&desc, None, Some(&mut staging)) }
        .map_err(|error| D3d11FrameContractError::WindowsApi(error.to_string()))?;
    let staging = staging.ok_or(D3d11FrameContractError::MissingStagingTexture)?;
    let source: ID3D11Resource = texture
        .cast()
        .map_err(|error| D3d11FrameContractError::WindowsApi(error.to_string()))?;
    let staging_resource: ID3D11Resource = staging
        .cast()
        .map_err(|error| D3d11FrameContractError::WindowsApi(error.to_string()))?;
    unsafe { context.CopyResource(&staging_resource, &source) };

    let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
    unsafe { context.Map(&staging_resource, 0, D3D11_MAP_READ, 0, Some(&mut mapped)) }
        .map_err(|error| D3d11FrameContractError::WindowsApi(error.to_string()))?;
    let staging_meta =
        match D3d11StagingReadbackMetadata::new(desc.Width, desc.Height, mapped.RowPitch, true) {
            Ok(metadata) => metadata,
            Err(error) => {
                unsafe { context.Unmap(&staging_resource, 0) };
                return Err(error);
            }
        };
    let bytes = unsafe {
        std::slice::from_raw_parts(mapped.pData.cast::<u8>(), staging_meta.byte_len).to_vec()
    };
    unsafe { context.Unmap(&staging_resource, 0) };
    let metadata = D3d11TextureFrameMetadata::new(
        D3d11FrameSource::DxgiDesktopDuplication,
        desc.Width,
        desc.Height,
        format,
    )
    .with_staging_readback(staging_meta)
    .with_frame_id(frame_id);
    D3d11TextureFrame::new(metadata, Some(bytes))
}

#[cfg(windows)]
pub fn readback_dxgi_d3d11_texture_2d_region_rgba(
    device: &ID3D11Device,
    context: &ID3D11DeviceContext,
    texture: &ID3D11Texture2D,
    crop: CropRect,
) -> D3d11FrameResult<Vec<u8>> {
    if crop.is_empty() {
        return Err(D3d11FrameContractError::EmptyDimensions {
            width: crop.width,
            height: crop.height,
        });
    }

    let mut desc = D3D11_TEXTURE2D_DESC::default();
    unsafe { texture.GetDesc(&mut desc) };
    let format = dxgi_texture_format(desc.Format)?;
    if crop.right() > desc.Width || crop.bottom() > desc.Height {
        return Err(D3d11FrameContractError::SelectedRegionOutsideFrame);
    }

    desc.Width = crop.width;
    desc.Height = crop.height;
    desc.Usage = D3D11_USAGE_STAGING;
    desc.BindFlags = 0;
    desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
    desc.MiscFlags = 0;

    let mut staging = None;
    unsafe { device.CreateTexture2D(&desc, None, Some(&mut staging)) }
        .map_err(|error| D3d11FrameContractError::WindowsApi(error.to_string()))?;
    let staging = staging.ok_or(D3d11FrameContractError::MissingStagingTexture)?;
    let source: ID3D11Resource = texture
        .cast()
        .map_err(|error| D3d11FrameContractError::WindowsApi(error.to_string()))?;
    let staging_resource: ID3D11Resource = staging
        .cast()
        .map_err(|error| D3d11FrameContractError::WindowsApi(error.to_string()))?;
    let source_box = D3D11_BOX {
        left: crop.x,
        top: crop.y,
        front: 0,
        right: crop.right(),
        bottom: crop.bottom(),
        back: 1,
    };
    unsafe {
        context.CopySubresourceRegion(&staging_resource, 0, 0, 0, 0, &source, 0, Some(&source_box))
    };

    let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
    unsafe { context.Map(&staging_resource, 0, D3D11_MAP_READ, 0, Some(&mut mapped)) }
        .map_err(|error| D3d11FrameContractError::WindowsApi(error.to_string()))?;
    let result = mapped_region_to_rgba(format, crop.width, crop.height, &mapped);
    unsafe { context.Unmap(&staging_resource, 0) };
    result
}

#[cfg(windows)]
fn dxgi_texture_format(
    format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT,
) -> D3d11FrameResult<D3d11TextureFrameFormat> {
    match format {
        windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM => {
            Ok(D3d11TextureFrameFormat::Bgra8Unorm)
        }
        windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_R8G8B8A8_UNORM => {
            Ok(D3d11TextureFrameFormat::Rgba8Unorm)
        }
        _ => Err(D3d11FrameContractError::UnsupportedTextureFormat(format!(
            "{:?}",
            format
        ))),
    }
}

#[cfg(windows)]
fn mapped_region_to_rgba(
    format: D3d11TextureFrameFormat,
    width: u32,
    height: u32,
    mapped: &D3D11_MAPPED_SUBRESOURCE,
) -> D3d11FrameResult<Vec<u8>> {
    let row_pitch =
        usize::try_from(mapped.RowPitch).map_err(|_| D3d11FrameContractError::PitchOverflow)?;
    let width = usize::try_from(width).map_err(|_| D3d11FrameContractError::PitchOverflow)?;
    let height = usize::try_from(height).map_err(|_| D3d11FrameContractError::PitchOverflow)?;
    let row_len = width
        .checked_mul(D3D11_RGBA_BYTES_PER_PIXEL as usize)
        .ok_or(D3d11FrameContractError::PitchOverflow)?;
    if row_pitch < row_len {
        return Err(D3d11FrameContractError::RowPitchTooSmall {
            row_pitch: mapped.RowPitch,
            minimum: u32::try_from(row_len).map_err(|_| D3d11FrameContractError::PitchOverflow)?,
        });
    }

    let output_len = row_len
        .checked_mul(height)
        .ok_or(D3d11FrameContractError::PitchOverflow)?;
    let source_len = row_pitch
        .checked_mul(height)
        .ok_or(D3d11FrameContractError::PitchOverflow)?;
    let source = unsafe { std::slice::from_raw_parts(mapped.pData.cast::<u8>(), source_len) };
    let mut rgba = Vec::with_capacity(output_len);
    for row in 0..height {
        let start = row
            .checked_mul(row_pitch)
            .ok_or(D3d11FrameContractError::PitchOverflow)?;
        let end = start
            .checked_add(row_len)
            .ok_or(D3d11FrameContractError::PitchOverflow)?;
        let row_bytes = source
            .get(start..end)
            .ok_or(D3d11FrameContractError::ByteLenTooSmall {
                byte_len: source.len(),
                minimum: end,
            })?;
        append_rgba_row(format, row_bytes, &mut rgba);
    }
    Ok(rgba)
}

#[cfg(windows)]
fn append_rgba_row(format: D3d11TextureFrameFormat, row_bytes: &[u8], rgba: &mut Vec<u8>) {
    match format {
        D3d11TextureFrameFormat::Rgba8Unorm => rgba.extend_from_slice(row_bytes),
        D3d11TextureFrameFormat::Bgra8Unorm => {
            for pixel in row_bytes.chunks_exact(D3D11_RGBA_BYTES_PER_PIXEL as usize) {
                rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
            }
        }
    }
}

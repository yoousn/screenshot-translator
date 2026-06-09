#[cfg(windows)]
use super::dxgi_readback::build_selected_png_contract_from_dxgi_texture;
#[cfg(windows)]
use super::output::{ImageBounds, SelectedImageContract, SelectionRect};
use super::MonitorCaptureBounds;

#[cfg(windows)]
use windows::Win32::Graphics::Direct3D11::{ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WgcSelectedReadbackError {
    MissingRequestedBounds,
    MissingTargetBounds,
    SelectionOutsideTarget,
    SelectionCoordinateOverflow,
    TextureReadback(String),
}

impl std::fmt::Display for WgcSelectedReadbackError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingRequestedBounds => {
                formatter.write_str("WGC selected readback requires requested bounds")
            }
            Self::MissingTargetBounds => {
                formatter.write_str("WGC selected readback requires target monitor bounds")
            }
            Self::SelectionOutsideTarget => formatter
                .write_str("WGC selected readback selection is outside target monitor bounds"),
            Self::SelectionCoordinateOverflow => formatter
                .write_str("WGC selected readback selection exceeds supported i32 coordinates"),
            Self::TextureReadback(reason) => {
                write!(formatter, "WGC selected texture readback failed: {reason}")
            }
        }
    }
}

impl std::error::Error for WgcSelectedReadbackError {}

pub type WgcSelectedReadbackResult<T> = Result<T, WgcSelectedReadbackError>;

pub fn selected_rect_from_wgc_monitor_bounds(
    requested: Option<MonitorCaptureBounds>,
    target: Option<MonitorCaptureBounds>,
) -> WgcSelectedReadbackResult<SelectionRect> {
    let requested = requested.ok_or(WgcSelectedReadbackError::MissingRequestedBounds)?;
    let target = target.ok_or(WgcSelectedReadbackError::MissingTargetBounds)?;
    let requested_right = requested
        .right()
        .ok_or(WgcSelectedReadbackError::SelectionOutsideTarget)?;
    let requested_bottom = requested
        .bottom()
        .ok_or(WgcSelectedReadbackError::SelectionOutsideTarget)?;
    let target_right = target
        .right()
        .ok_or(WgcSelectedReadbackError::SelectionOutsideTarget)?;
    let target_bottom = target
        .bottom()
        .ok_or(WgcSelectedReadbackError::SelectionOutsideTarget)?;
    if requested.origin_x < target.origin_x
        || requested.origin_y < target.origin_y
        || requested_right > target_right
        || requested_bottom > target_bottom
    {
        return Err(WgcSelectedReadbackError::SelectionOutsideTarget);
    }
    Ok(SelectionRect::new(
        requested.origin_x - target.origin_x,
        requested.origin_y - target.origin_y,
        i32::try_from(requested.width)
            .map_err(|_| WgcSelectedReadbackError::SelectionCoordinateOverflow)?,
        i32::try_from(requested.height)
            .map_err(|_| WgcSelectedReadbackError::SelectionCoordinateOverflow)?,
    ))
}

#[cfg(windows)]
pub fn build_selected_png_contract_from_wgc_texture(
    device: &ID3D11Device,
    context: &ID3D11DeviceContext,
    texture: &ID3D11Texture2D,
    requested: Option<MonitorCaptureBounds>,
    target: Option<MonitorCaptureBounds>,
) -> WgcSelectedReadbackResult<SelectedImageContract> {
    let selection = selected_rect_from_wgc_monitor_bounds(requested, target)?;
    let target = target.ok_or(WgcSelectedReadbackError::MissingTargetBounds)?;
    build_selected_png_contract_from_dxgi_texture(
        device,
        context,
        texture,
        ImageBounds::new(target.width, target.height),
        selection,
    )
    .map_err(|error| WgcSelectedReadbackError::TextureReadback(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_rect_is_monitor_local() {
        let requested = MonitorCaptureBounds::new(110, 220, 320, 180);
        let target = MonitorCaptureBounds::new(100, 200, 1920, 1080);

        let rect = selected_rect_from_wgc_monitor_bounds(Some(requested), Some(target)).unwrap();

        assert_eq!(rect, SelectionRect::new(10, 20, 320, 180));
    }

    #[test]
    fn selected_rect_rejects_outside_target() {
        let requested = MonitorCaptureBounds::new(90, 220, 320, 180);
        let target = MonitorCaptureBounds::new(100, 200, 1920, 1080);

        let error =
            selected_rect_from_wgc_monitor_bounds(Some(requested), Some(target)).unwrap_err();

        assert_eq!(error, WgcSelectedReadbackError::SelectionOutsideTarget);
    }
}

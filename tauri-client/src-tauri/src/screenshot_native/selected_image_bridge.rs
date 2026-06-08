use base64::{prelude::BASE64_STANDARD, Engine};
use serde::{Deserialize, Serialize};

use super::{
    ImageBounds, OutputAction, OutputBridgeContract, OutputBridgeTarget, OutputImageFormat,
    RgbaFrame, SelectedImageContract, SelectionRect,
};

const PNG_MIME_TYPE: &str = "image/png";
const PNG_DATA_URL_PREFIX: &str = "data:image/png;base64,";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedImageBridgeContract {
    pub action: OutputAction,
    pub target: OutputBridgeTarget,
    pub format: OutputImageFormat,
    pub mime_type: String,
    pub image: SelectedImageContract,
    pub png_base64: String,
    pub data_url: String,
}

impl SelectedImageBridgeContract {
    pub fn description(&self) -> SelectedImageBridgeDescription {
        SelectedImageBridgeDescription {
            action: self.action,
            target: self.target,
            format: self.format,
            mime_type: self.mime_type.clone(),
            rect: self.image.rect,
            crop: self.image.crop,
            source_width: self.image.source_width,
            source_height: self.image.source_height,
            png_byte_len: self.image.byte_len(),
            base64_len: self.png_base64.len(),
            data_url_len: self.data_url.len(),
            was_clamped: self.image.was_clamped,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedImageBridgeDescription {
    pub action: OutputAction,
    pub target: OutputBridgeTarget,
    pub format: OutputImageFormat,
    pub mime_type: String,
    pub rect: SelectionRect,
    pub crop: super::CropRect,
    pub source_width: u32,
    pub source_height: u32,
    pub png_byte_len: usize,
    pub base64_len: usize,
    pub data_url_len: usize,
    pub was_clamped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectedImageBridgeError {
    InvalidFrame(String),
    EmptySelection,
    SelectionOutsideFrame,
    UnsupportedAction(OutputAction),
    PngEncodeFailed(String),
}

impl std::fmt::Display for SelectedImageBridgeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFrame(reason) => write!(formatter, "invalid RGBA frame: {reason}"),
            Self::EmptySelection => formatter.write_str("selection is empty"),
            Self::SelectionOutsideFrame => formatter.write_str("selection is outside the frame"),
            Self::UnsupportedAction(action) => {
                write!(formatter, "unsupported output action: {action:?}")
            }
            Self::PngEncodeFailed(reason) => write!(formatter, "PNG encode failed: {reason}"),
        }
    }
}

impl std::error::Error for SelectedImageBridgeError {}

pub type SelectedImageBridgeResult<T> = Result<T, SelectedImageBridgeError>;

pub fn build_selected_png_contract(
    frame: &RgbaFrame,
    selection: SelectionRect,
) -> SelectedImageBridgeResult<SelectedImageContract> {
    frame
        .validate()
        .map_err(|error| SelectedImageBridgeError::InvalidFrame(error.to_string()))?;

    let bounds = ImageBounds::new(frame.width, frame.height);
    let clamped = selection
        .clamp_to(bounds)
        .ok_or_else(|| bridge_selection_error(selection, bounds))?;
    let cropped_rgba = crop_rgba(frame, clamped.crop)?;
    let png_bytes = encode_rgba_png(&cropped_rgba, clamped.crop.width, clamped.crop.height)?;

    Ok(SelectedImageContract::new(clamped, png_bytes, bounds))
}

pub fn build_selected_image_bridge_contract(
    action: OutputAction,
    frame: &RgbaFrame,
    selection: SelectionRect,
) -> SelectedImageBridgeResult<SelectedImageBridgeContract> {
    let image = build_selected_png_contract(frame, selection)?;
    let target = action
        .bridge_target()
        .ok_or(SelectedImageBridgeError::UnsupportedAction(action))?;
    let png_base64 = BASE64_STANDARD.encode(&image.png_bytes);
    let data_url = format!("{PNG_DATA_URL_PREFIX}{png_base64}");

    Ok(SelectedImageBridgeContract {
        action,
        target,
        format: OutputImageFormat::Png,
        mime_type: PNG_MIME_TYPE.to_string(),
        image,
        png_base64,
        data_url,
    })
}

pub fn build_output_bridge_contract(
    action: OutputAction,
    frame: &RgbaFrame,
    selection: SelectionRect,
) -> SelectedImageBridgeResult<OutputBridgeContract> {
    let image = build_selected_png_contract(frame, selection)?;
    OutputBridgeContract::new(action, image)
        .ok_or(SelectedImageBridgeError::UnsupportedAction(action))
}

pub fn describe_selected_image_bridge(
    contract: &SelectedImageBridgeContract,
) -> SelectedImageBridgeDescription {
    contract.description()
}

fn bridge_selection_error(
    selection: SelectionRect,
    bounds: ImageBounds,
) -> SelectedImageBridgeError {
    if !selection.normalized().is_valid() || bounds.is_empty() {
        SelectedImageBridgeError::EmptySelection
    } else {
        SelectedImageBridgeError::SelectionOutsideFrame
    }
}

fn crop_rgba(frame: &RgbaFrame, crop: super::CropRect) -> SelectedImageBridgeResult<Vec<u8>> {
    if crop.is_empty() {
        return Err(SelectedImageBridgeError::EmptySelection);
    }

    let source_width = usize::try_from(frame.width)
        .map_err(|_| SelectedImageBridgeError::InvalidFrame("width overflows usize".into()))?;
    let crop_x = usize::try_from(crop.x)
        .map_err(|_| SelectedImageBridgeError::InvalidFrame("crop x overflows usize".into()))?;
    let crop_y = usize::try_from(crop.y)
        .map_err(|_| SelectedImageBridgeError::InvalidFrame("crop y overflows usize".into()))?;
    let crop_width = usize::try_from(crop.width)
        .map_err(|_| SelectedImageBridgeError::InvalidFrame("crop width overflows usize".into()))?;
    let crop_height = usize::try_from(crop.height).map_err(|_| {
        SelectedImageBridgeError::InvalidFrame("crop height overflows usize".into())
    })?;
    let row_bytes = crop_width
        .checked_mul(RgbaFrame::BYTES_PER_PIXEL)
        .ok_or_else(|| SelectedImageBridgeError::InvalidFrame("crop row bytes overflow".into()))?;
    let source_stride = source_width
        .checked_mul(RgbaFrame::BYTES_PER_PIXEL)
        .ok_or_else(|| SelectedImageBridgeError::InvalidFrame("source stride overflow".into()))?;
    let output_len = row_bytes.checked_mul(crop_height).ok_or_else(|| {
        SelectedImageBridgeError::InvalidFrame("crop byte length overflow".into())
    })?;

    let mut output = Vec::with_capacity(output_len);
    for row in 0..crop_height {
        let source_row = crop_y
            .checked_add(row)
            .ok_or_else(|| SelectedImageBridgeError::InvalidFrame("crop row overflow".into()))?;
        let start = source_row
            .checked_mul(source_stride)
            .and_then(|offset| offset.checked_add(crop_x * RgbaFrame::BYTES_PER_PIXEL))
            .ok_or_else(|| SelectedImageBridgeError::InvalidFrame("crop offset overflow".into()))?;
        let end = start
            .checked_add(row_bytes)
            .ok_or_else(|| SelectedImageBridgeError::InvalidFrame("crop slice overflow".into()))?;
        let row_bytes = frame.bytes.get(start..end).ok_or_else(|| {
            SelectedImageBridgeError::InvalidFrame("crop slice is out of range".into())
        })?;
        output.extend_from_slice(row_bytes);
    }

    Ok(output)
}

fn encode_rgba_png(rgba: &[u8], width: u32, height: u32) -> SelectedImageBridgeResult<Vec<u8>> {
    let mut buffer = std::io::Cursor::new(Vec::new());
    let encoder = screenshots::image::codecs::png::PngEncoder::new_with_quality(
        &mut buffer,
        screenshots::image::codecs::png::CompressionType::Fast,
        screenshots::image::codecs::png::FilterType::NoFilter,
    );
    screenshots::image::ImageEncoder::write_image(
        encoder,
        rgba,
        width,
        height,
        screenshots::image::ColorType::Rgba8.into(),
    )
    .map_err(|error| SelectedImageBridgeError::PngEncodeFailed(error.to_string()))?;
    Ok(buffer.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_base64_png_bridge_for_clamped_selection() {
        let frame = RgbaFrame::new(
            2,
            2,
            vec![
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
            ],
        )
        .expect("valid test frame");

        let contract = build_selected_image_bridge_contract(
            OutputAction::Copy,
            &frame,
            SelectionRect::new(1, 1, 3, 3),
        )
        .expect("bridge contract");

        assert_eq!(contract.target, OutputBridgeTarget::Clipboard);
        assert_eq!(contract.image.crop.width, 1);
        assert_eq!(contract.image.crop.height, 1);
        assert!(contract.image.was_clamped);
        assert!(contract.data_url.starts_with(PNG_DATA_URL_PREFIX));
        assert!(!contract.png_base64.is_empty());
        assert!(!contract.image.png_bytes.is_empty());
    }

    #[test]
    fn rejects_record_action_without_side_effects() {
        let frame = RgbaFrame::new(1, 1, vec![0, 0, 0, 0]).expect("valid test frame");
        let error = build_selected_image_bridge_contract(
            OutputAction::Record,
            &frame,
            SelectionRect::new(0, 0, 1, 1),
        )
        .expect_err("record is not an image bridge action");

        assert_eq!(
            error,
            SelectedImageBridgeError::UnsupportedAction(OutputAction::Record)
        );
    }
}

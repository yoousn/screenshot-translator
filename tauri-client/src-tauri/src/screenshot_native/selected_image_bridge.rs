use base64::{prelude::BASE64_STANDARD, Engine};
use serde::{Deserialize, Serialize};

use super::output::SelectedReadbackContract;
use super::{
    ImageBounds, OutputAction, OutputBridgeContract, OutputBridgeTarget, OutputImageFormat,
    RgbaFrame, SelectedImageContract, SelectionRect,
};

const PNG_MIME_TYPE: &str = "image/png";
const PNG_DATA_URL_PREFIX: &str = "data:image/png;base64,";
const PNG_SIGNATURE: &[u8; 8] = &[137, 80, 78, 71, 13, 10, 26, 10];
const IMAGE_BRIDGE_ACTIONS: [OutputAction; 4] = [
    OutputAction::Copy,
    OutputAction::SaveAs,
    OutputAction::Ocr,
    OutputAction::Translate,
];

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
            selected_only: self.image.is_selected_only_png(),
            readback: self.image.readback_contract(),
            was_clamped: self.image.was_clamped,
        }
    }

    pub fn diagnostics(&self) -> SelectedImageBridgeDiagnostics {
        SelectedImageBridgeDiagnostics {
            description: self.description(),
            png_signature_valid: self.image.png_bytes.starts_with(PNG_SIGNATURE),
            data_url_prefix_valid: self.data_url.starts_with(PNG_DATA_URL_PREFIX),
            base64_matches_png: BASE64_STANDARD.encode(&self.image.png_bytes) == self.png_base64,
            selected_only_png: self.image.is_selected_only_png(),
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
    pub selected_only: bool,
    pub readback: Option<SelectedReadbackContract>,
    pub was_clamped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedImageBridgeDiagnostics {
    pub description: SelectedImageBridgeDescription,
    pub png_signature_valid: bool,
    pub data_url_prefix_valid: bool,
    pub base64_matches_png: bool,
    pub selected_only_png: bool,
}

impl SelectedImageBridgeDiagnostics {
    pub fn is_valid(&self) -> bool {
        self.png_signature_valid
            && self.data_url_prefix_valid
            && self.base64_matches_png
            && self.selected_only_png
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedOutputBridgeDryRunDiagnostic {
    pub action: OutputAction,
    pub target: OutputBridgeTarget,
    pub format: OutputImageFormat,
    pub readback: Option<SelectedReadbackContract>,
    pub png_byte_len: usize,
    pub target_matches_action: bool,
    pub png_signature_valid: bool,
    pub selected_only_png: bool,
}

impl SelectedOutputBridgeDryRunDiagnostic {
    pub fn is_valid(&self) -> bool {
        self.target_matches_action && self.png_signature_valid && self.selected_only_png
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedOutputBridgeDryRunReport {
    pub image: SelectedImageBridgeDescription,
    pub diagnostics: Vec<SelectedOutputBridgeDryRunDiagnostic>,
}

impl SelectedOutputBridgeDryRunReport {
    pub fn is_valid(&self) -> bool {
        self.diagnostics.len() == IMAGE_BRIDGE_ACTIONS.len()
            && self
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.is_valid())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectedImageBridgeError {
    InvalidFrame(String),
    EmptySelection,
    SelectionOutsideFrame,
    UnsupportedAction(OutputAction),
    ReadbackContractOverflow,
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
            Self::ReadbackContractOverflow => {
                formatter.write_str("selected-region readback contract overflowed")
            }
            Self::PngEncodeFailed(reason) => write!(formatter, "PNG encode failed: {reason}"),
        }
    }
}

impl std::error::Error for SelectedImageBridgeError {}

pub type SelectedImageBridgeResult<T> = Result<T, SelectedImageBridgeError>;

pub trait SelectedRegionReadbackSource {
    fn bounds(&self) -> ImageBounds;

    fn validate_readback(&self) -> SelectedImageBridgeResult<()>;

    fn read_selected_rgba(&self, crop: super::CropRect) -> SelectedImageBridgeResult<Vec<u8>>;
}

impl SelectedRegionReadbackSource for RgbaFrame {
    fn bounds(&self) -> ImageBounds {
        ImageBounds::new(self.width, self.height)
    }

    fn validate_readback(&self) -> SelectedImageBridgeResult<()> {
        self.validate()
            .map_err(|error| SelectedImageBridgeError::InvalidFrame(error.to_string()))
    }

    fn read_selected_rgba(&self, crop: super::CropRect) -> SelectedImageBridgeResult<Vec<u8>> {
        crop_rgba(self, crop)
    }
}

pub fn build_selected_png_contract(
    frame: &RgbaFrame,
    selection: SelectionRect,
) -> SelectedImageBridgeResult<SelectedImageContract> {
    build_selected_png_contract_from_source(frame, selection)
}

pub fn build_selected_png_contract_from_source(
    source: &impl SelectedRegionReadbackSource,
    selection: SelectionRect,
) -> SelectedImageBridgeResult<SelectedImageContract> {
    source.validate_readback()?;

    let bounds = source.bounds();
    let clamped = selection
        .clamp_to(bounds)
        .ok_or_else(|| bridge_selection_error(selection, bounds))?;
    SelectedReadbackContract::new(clamped, bounds)
        .filter(|readback| readback.is_selected_only())
        .ok_or(SelectedImageBridgeError::ReadbackContractOverflow)?;
    let cropped_rgba = source.read_selected_rgba(clamped.crop)?;
    let png_bytes = encode_rgba_png(&cropped_rgba, clamped.crop.width, clamped.crop.height)?;

    Ok(SelectedImageContract::new(clamped, png_bytes, bounds))
}

pub fn build_selected_image_bridge_contract(
    action: OutputAction,
    frame: &RgbaFrame,
    selection: SelectionRect,
) -> SelectedImageBridgeResult<SelectedImageBridgeContract> {
    let target = action
        .bridge_target()
        .ok_or(SelectedImageBridgeError::UnsupportedAction(action))?;
    let image = build_selected_png_contract(frame, selection)?;
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
    if action.bridge_target().is_none() {
        return Err(SelectedImageBridgeError::UnsupportedAction(action));
    }
    let image = build_selected_png_contract(frame, selection)?;
    OutputBridgeContract::new(action, image)
        .ok_or(SelectedImageBridgeError::UnsupportedAction(action))
}

pub fn describe_selected_image_bridge(
    contract: &SelectedImageBridgeContract,
) -> SelectedImageBridgeDescription {
    contract.description()
}

pub fn diagnose_selected_image_bridge(
    contract: &SelectedImageBridgeContract,
) -> SelectedImageBridgeDiagnostics {
    contract.diagnostics()
}

pub fn diagnose_selected_output_bridge_contract(
    action: OutputAction,
    image: &SelectedImageContract,
) -> SelectedImageBridgeResult<SelectedOutputBridgeDryRunDiagnostic> {
    let target = action
        .bridge_target()
        .ok_or(SelectedImageBridgeError::UnsupportedAction(action))?;
    let contract = OutputBridgeContract::new(action, image.clone())
        .ok_or(SelectedImageBridgeError::UnsupportedAction(action))?;

    Ok(SelectedOutputBridgeDryRunDiagnostic {
        action,
        target: contract.target,
        format: image.image_format(),
        readback: image.readback_contract(),
        png_byte_len: image.byte_len(),
        target_matches_action: contract.target == target,
        png_signature_valid: image.png_bytes.starts_with(PNG_SIGNATURE),
        selected_only_png: image.is_selected_only_png(),
    })
}

pub fn dry_run_selected_output_bridge_contracts(
    image: &SelectedImageContract,
) -> SelectedImageBridgeResult<SelectedOutputBridgeDryRunReport> {
    let diagnostics = IMAGE_BRIDGE_ACTIONS
        .into_iter()
        .map(|action| diagnose_selected_output_bridge_contract(action, image))
        .collect::<SelectedImageBridgeResult<Vec<_>>>()?;

    Ok(SelectedOutputBridgeDryRunReport {
        image: describe_selected_png_evidence(image),
        diagnostics,
    })
}

pub fn dry_run_selected_output_bridges_from_source(
    source: &impl SelectedRegionReadbackSource,
    selection: SelectionRect,
) -> SelectedImageBridgeResult<SelectedOutputBridgeDryRunReport> {
    let image = build_selected_png_contract_from_source(source, selection)?;
    dry_run_selected_output_bridge_contracts(&image)
}

pub fn describe_selected_png_evidence(
    image: &SelectedImageContract,
) -> SelectedImageBridgeDescription {
    SelectedImageBridgeDescription {
        action: OutputAction::Copy,
        target: OutputBridgeTarget::Clipboard,
        format: image.image_format(),
        mime_type: PNG_MIME_TYPE.to_string(),
        rect: image.rect,
        crop: image.crop,
        source_width: image.source_width,
        source_height: image.source_height,
        png_byte_len: image.byte_len(),
        base64_len: 0,
        data_url_len: 0,
        selected_only: image.is_selected_only_png(),
        readback: image.readback_contract(),
        was_clamped: image.was_clamped,
    }
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

    if crop.right() > frame.width || crop.bottom() > frame.height {
        return Err(SelectedImageBridgeError::SelectionOutsideFrame);
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
        assert_eq!(contract.image.readback_contract().unwrap().rgba_byte_len, 4);
        assert!(contract.diagnostics().is_valid());
    }

    #[test]
    fn selected_png_builder_accepts_readback_source_trait() {
        let frame =
            RgbaFrame::new(2, 1, vec![255, 0, 0, 255, 0, 255, 0, 255]).expect("valid test frame");

        let image = build_selected_png_contract_from_source(&frame, SelectionRect::new(1, 0, 2, 1))
            .expect("selected image from source");

        assert_eq!(image.crop.width, 1);
        assert_eq!(image.crop.height, 1);
        assert_eq!(image.source_width, 2);
        assert_eq!(image.readback_contract().unwrap().rgba_byte_len, 4);
        assert!(image.is_selected_only_png());
    }

    #[test]
    fn dry_run_validates_all_image_bridge_actions_from_png_evidence() {
        let frame =
            RgbaFrame::new(2, 1, vec![255, 0, 0, 255, 0, 255, 0, 255]).expect("valid test frame");
        let image = build_selected_png_contract(&frame, SelectionRect::new(0, 0, 2, 1))
            .expect("selected png evidence");

        let report = dry_run_selected_output_bridge_contracts(&image).expect("dry-run report");

        assert!(report.is_valid());
        assert_eq!(report.image.png_byte_len, image.byte_len());
        assert_eq!(report.diagnostics.len(), 4);
        assert_eq!(report.diagnostics[0].action, OutputAction::Copy);
        assert_eq!(report.diagnostics[0].target, OutputBridgeTarget::Clipboard);
        assert_eq!(report.diagnostics[1].action, OutputAction::SaveAs);
        assert_eq!(report.diagnostics[1].target, OutputBridgeTarget::File);
        assert_eq!(report.diagnostics[2].action, OutputAction::Ocr);
        assert_eq!(report.diagnostics[2].target, OutputBridgeTarget::Ocr);
        assert_eq!(report.diagnostics[3].action, OutputAction::Translate);
        assert_eq!(
            report.diagnostics[3].target,
            OutputBridgeTarget::Translation
        );
    }

    #[test]
    fn dry_run_from_readback_source_builds_selected_png_without_output_side_effects() {
        let frame = RgbaFrame::new(
            2,
            2,
            vec![
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
            ],
        )
        .expect("valid test frame");

        let report =
            dry_run_selected_output_bridges_from_source(&frame, SelectionRect::new(1, 1, 4, 4))
                .expect("dry-run from source");

        assert!(report.is_valid());
        assert_eq!(report.image.crop.width, 1);
        assert_eq!(report.image.crop.height, 1);
        assert!(report.image.was_clamped);
        assert!(report.diagnostics.iter().all(|diagnostic| diagnostic
            .readback
            .is_some_and(|readback| readback.selected_only)));
    }

    #[test]
    fn diagnostic_rejects_record_action_without_building_output_contract() {
        let frame = RgbaFrame::new(1, 1, vec![0, 0, 0, 0]).expect("valid test frame");
        let image = build_selected_png_contract(&frame, SelectionRect::new(0, 0, 1, 1))
            .expect("selected png evidence");
        let error = diagnose_selected_output_bridge_contract(OutputAction::Record, &image)
            .expect_err("record is not an output bridge action");

        assert_eq!(
            error,
            SelectedImageBridgeError::UnsupportedAction(OutputAction::Record)
        );
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

    #[test]
    fn output_bridge_rejects_record_before_png_work() {
        let frame = RgbaFrame::new(1, 1, vec![0, 0, 0, 0]).expect("valid test frame");
        let error = build_output_bridge_contract(
            OutputAction::Record,
            &frame,
            SelectionRect::new(0, 0, 1, 1),
        )
        .expect_err("record is not an output bridge action");

        assert_eq!(
            error,
            SelectedImageBridgeError::UnsupportedAction(OutputAction::Record)
        );
    }
}

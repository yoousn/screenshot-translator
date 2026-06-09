use std::borrow::Cow;

use arboard::{Clipboard, ImageData};
use serde::{Deserialize, Serialize};

use super::selected_output_effects::{SelectedOutputEffectError, SelectedOutputEffectSink};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ClipboardImageVerification {
    pub expected_width: usize,
    pub expected_height: usize,
    pub actual_width: usize,
    pub actual_height: usize,
    pub expected_byte_len: usize,
    pub actual_byte_len: usize,
    pub expected_rgba_fingerprint: String,
    pub actual_rgba_fingerprint: String,
    pub dimensions_match: bool,
    pub bytes_match: bool,
}

impl ClipboardImageVerification {
    pub(crate) fn confirmed(&self) -> bool {
        self.dimensions_match && self.bytes_match
    }
}

#[derive(Debug, Default)]
pub(crate) struct ArboardSelectedOutputEffectSink;

impl ArboardSelectedOutputEffectSink {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl SelectedOutputEffectSink for ArboardSelectedOutputEffectSink {
    fn copy_png_to_clipboard(&mut self, png_bytes: &[u8]) -> Result<(), SelectedOutputEffectError> {
        let image = decode_png_for_clipboard(png_bytes)?;
        let mut clipboard = Clipboard::new().map_err(|error| {
            SelectedOutputEffectError::ClipboardWriteFailed(format!(
                "initialize clipboard failed: {error}"
            ))
        })?;
        clipboard.set_image(image).map_err(|error| {
            SelectedOutputEffectError::ClipboardWriteFailed(format!(
                "set clipboard image failed: {error}"
            ))
        })
    }
}

#[derive(Debug, Default)]
pub(crate) struct VerifyingArboardSelectedOutputEffectSink {
    verification: Option<ClipboardImageVerification>,
    readback_attempted: bool,
}

impl VerifyingArboardSelectedOutputEffectSink {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn verification(&self) -> Option<&ClipboardImageVerification> {
        self.verification.as_ref()
    }

    pub(crate) fn readback_attempted(&self) -> bool {
        self.readback_attempted
    }
}

impl SelectedOutputEffectSink for VerifyingArboardSelectedOutputEffectSink {
    fn copy_png_to_clipboard(&mut self, png_bytes: &[u8]) -> Result<(), SelectedOutputEffectError> {
        let expected = decode_png_for_clipboard(png_bytes)?;
        let mut clipboard = Clipboard::new().map_err(|error| {
            SelectedOutputEffectError::ClipboardWriteFailed(format!(
                "initialize clipboard failed: {error}"
            ))
        })?;
        clipboard.set_image(expected.clone()).map_err(|error| {
            SelectedOutputEffectError::ClipboardWriteFailed(format!(
                "set clipboard image failed: {error}"
            ))
        })?;
        self.readback_attempted = true;
        let actual = clipboard.get_image().map_err(|error| {
            SelectedOutputEffectError::ClipboardWriteFailed(format!(
                "read clipboard image verification failed: {error}"
            ))
        })?;
        let verification = verify_clipboard_image_data(&expected, &actual);
        let confirmed = verification.confirmed();
        self.verification = Some(verification);
        if confirmed {
            Ok(())
        } else {
            Err(SelectedOutputEffectError::ClipboardWriteFailed(
                "clipboard image verification mismatch after write".to_string(),
            ))
        }
    }
}

fn decode_png_for_clipboard(
    png_bytes: &[u8],
) -> Result<ImageData<'static>, SelectedOutputEffectError> {
    let image = image::load_from_memory_with_format(png_bytes, image::ImageFormat::Png)
        .map_err(|error| {
            SelectedOutputEffectError::ClipboardWriteFailed(format!(
                "parse selected PNG failed: {error}"
            ))
        })?
        .to_rgba8();
    let (width, height) = image.dimensions();

    Ok(ImageData {
        width: width as usize,
        height: height as usize,
        bytes: Cow::Owned(image.into_raw()),
    })
}

fn verify_clipboard_image_data(
    expected: &ImageData<'_>,
    actual: &ImageData<'_>,
) -> ClipboardImageVerification {
    let expected_bytes = expected.bytes.as_ref();
    let actual_bytes = actual.bytes.as_ref();
    ClipboardImageVerification {
        expected_width: expected.width,
        expected_height: expected.height,
        actual_width: actual.width,
        actual_height: actual.height,
        expected_byte_len: expected_bytes.len(),
        actual_byte_len: actual_bytes.len(),
        expected_rgba_fingerprint: rgba_fingerprint(expected_bytes),
        actual_rgba_fingerprint: rgba_fingerprint(actual_bytes),
        dimensions_match: expected.width == actual.width && expected.height == actual.height,
        bytes_match: expected_bytes == actual_bytes,
    }
}

fn rgba_fingerprint(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn copy_png_bytes() -> Vec<u8> {
        let frame = super::super::RgbaFrame::new(2, 1, vec![255, 0, 0, 255, 0, 255, 0, 255])
            .expect("valid test frame");
        super::super::selected_image_bridge::build_selected_image_bridge_contract(
            super::super::OutputAction::Copy,
            &frame,
            super::super::SelectionRect::new(0, 0, 2, 1),
        )
        .expect("copy bridge contract")
        .image
        .png_bytes
    }

    #[test]
    fn clipboard_png_decoder_preserves_dimensions_and_rgba_bytes() {
        let png_bytes = copy_png_bytes();
        let image = decode_png_for_clipboard(&png_bytes).expect("clipboard image");

        assert_eq!(image.width, 2);
        assert_eq!(image.height, 1);
        assert_eq!(image.bytes.as_ref(), &[255, 0, 0, 255, 0, 255, 0, 255]);
    }

    #[test]
    fn clipboard_png_decoder_rejects_corrupted_png() {
        let error = decode_png_for_clipboard(b"not a png").expect_err("invalid png rejected");

        assert!(matches!(
            error,
            SelectedOutputEffectError::ClipboardWriteFailed(message)
                if message.contains("parse selected PNG failed")
        ));
    }

    #[test]
    fn clipboard_image_verification_confirms_matching_dimensions_and_bytes() {
        let expected = ImageData {
            width: 2,
            height: 1,
            bytes: Cow::Owned(vec![255, 0, 0, 255, 0, 255, 0, 255]),
        };
        let actual = ImageData {
            width: 2,
            height: 1,
            bytes: Cow::Owned(vec![255, 0, 0, 255, 0, 255, 0, 255]),
        };
        let verification = verify_clipboard_image_data(&expected, &actual);

        assert_eq!(verification.expected_width, 2);
        assert_eq!(verification.expected_height, 1);
        assert_eq!(verification.actual_width, 2);
        assert_eq!(verification.actual_height, 1);
        assert_eq!(verification.expected_byte_len, 8);
        assert_eq!(verification.actual_byte_len, 8);
        assert_eq!(
            verification.expected_rgba_fingerprint,
            verification.actual_rgba_fingerprint
        );
        assert!(verification
            .expected_rgba_fingerprint
            .starts_with("fnv1a64:"));
        assert!(verification.dimensions_match);
        assert!(verification.bytes_match);
        assert!(verification.confirmed());
    }

    #[test]
    fn clipboard_image_verification_rejects_dimension_or_byte_mismatch() {
        let expected = ImageData {
            width: 2,
            height: 1,
            bytes: Cow::Owned(vec![255, 0, 0, 255, 0, 255, 0, 255]),
        };
        let actual = ImageData {
            width: 1,
            height: 2,
            bytes: Cow::Owned(vec![255, 0, 0, 255, 0, 0, 255, 255]),
        };
        let verification = verify_clipboard_image_data(&expected, &actual);

        assert!(!verification.dimensions_match);
        assert!(!verification.bytes_match);
        assert_ne!(
            verification.expected_rgba_fingerprint,
            verification.actual_rgba_fingerprint
        );
        assert!(!verification.confirmed());
    }
}

use base64::{prelude::BASE64_STANDARD, Engine};

use super::output::SelectedImageContract;
use super::selected_image_bridge::SelectedImageBridgeContract;
use super::selected_output_effects::{
    perform_selected_output_effect_with_sink, SelectedOutputEffectError,
    SelectedOutputEffectReceipt, SelectedOutputEffectRequest, SelectedOutputEffectSink,
};
use super::{OutputAction, OutputBridgeTarget, OutputImageFormat};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DxgiSelectedOutputClipboardAcceptanceReceipt {
    pub effect: SelectedOutputEffectReceipt,
    pub diagnostic_only: bool,
    pub readiness_changed: bool,
    pub persistent_handle_exposed: bool,
    pub sink: &'static str,
}

impl DxgiSelectedOutputClipboardAcceptanceReceipt {
    pub fn proves_fake_sink_copy(&self) -> bool {
        self.effect.is_copy_only()
            && self.diagnostic_only
            && !self.readiness_changed
            && !self.persistent_handle_exposed
            && self.sink == "provided-sink"
    }
}

pub fn accept_dxgi_selected_output_clipboard_with_sink(
    image: SelectedImageContract,
    explicit_opt_in: bool,
    sink: &mut impl SelectedOutputEffectSink,
) -> Result<DxgiSelectedOutputClipboardAcceptanceReceipt, SelectedOutputEffectError> {
    let png_base64 = BASE64_STANDARD.encode(&image.png_bytes);
    let contract = SelectedImageBridgeContract {
        action: OutputAction::Copy,
        target: OutputBridgeTarget::Clipboard,
        format: OutputImageFormat::Png,
        mime_type: "image/png".to_string(),
        data_url: format!("data:image/png;base64,{png_base64}"),
        png_base64,
        image,
    };
    let effect = perform_selected_output_effect_with_sink(
        &contract,
        SelectedOutputEffectRequest { explicit_opt_in },
        sink,
    )?;

    Ok(DxgiSelectedOutputClipboardAcceptanceReceipt {
        effect,
        diagnostic_only: true,
        readiness_changed: false,
        persistent_handle_exposed: false,
        sink: "provided-sink",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screenshot_native::output::{
        ClampedSelectionRect, CropRect, ImageBounds, SelectionRect,
    };

    #[derive(Default)]
    struct FakeClipboardSink {
        calls: usize,
        last_png_len: usize,
    }

    impl SelectedOutputEffectSink for FakeClipboardSink {
        fn copy_png_to_clipboard(
            &mut self,
            png_bytes: &[u8],
        ) -> Result<(), SelectedOutputEffectError> {
            self.calls += 1;
            self.last_png_len = png_bytes.len();
            Ok(())
        }
    }

    fn selected_png() -> SelectedImageContract {
        let png = vec![137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 0];
        SelectedImageContract::new(
            ClampedSelectionRect {
                requested: SelectionRect::new(2, 3, 4, 5),
                crop: CropRect {
                    x: 2,
                    y: 3,
                    width: 4,
                    height: 5,
                },
                was_clamped: false,
            },
            png,
            ImageBounds::new(20, 20),
        )
    }

    #[test]
    fn fake_sink_acceptance_proves_dxgi_selected_png_can_copy() {
        let mut sink = FakeClipboardSink::default();
        let receipt =
            accept_dxgi_selected_output_clipboard_with_sink(selected_png(), true, &mut sink)
                .expect("fake clipboard acceptance");

        assert!(receipt.proves_fake_sink_copy());
        assert_eq!(sink.calls, 1);
        assert_eq!(sink.last_png_len, receipt.effect.png_byte_len);
        assert!(receipt.effect.selected_only_png);
        assert!(receipt.effect.copied_to_clipboard);
        assert!(!receipt.effect.save_invoked);
        assert!(!receipt.effect.ocr_invoked);
        assert!(!receipt.effect.translation_invoked);
    }

    #[test]
    fn fake_sink_acceptance_requires_explicit_opt_in() {
        let mut sink = FakeClipboardSink::default();
        let error =
            accept_dxgi_selected_output_clipboard_with_sink(selected_png(), false, &mut sink)
                .expect_err("explicit opt-in required");

        assert_eq!(error, SelectedOutputEffectError::ExplicitOptInRequired);
        assert_eq!(sink.calls, 0);
    }
}

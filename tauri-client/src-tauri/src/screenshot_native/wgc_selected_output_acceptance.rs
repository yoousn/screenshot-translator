use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};

use super::output::SelectedImageContract;
use super::selected_image_bridge::SelectedImageBridgeContract;
use super::selected_output_effects::{
    perform_selected_output_effect_with_sink, SelectedOutputEffectError,
    SelectedOutputEffectReceipt, SelectedOutputEffectRequest, SelectedOutputEffectSink,
};
use super::{OutputAction, OutputBridgeTarget, OutputImageFormat};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WgcSelectedOutputFakeSinkAcceptanceReceipt {
    pub source: &'static str,
    pub diagnostic_only: bool,
    pub readiness_changed: bool,
    pub alt_a_changed: bool,
    pub persistent_handle_exposed: bool,
    pub wgc_selected_png_evidence_present: bool,
    pub fake_sink_copy_accepted: bool,
    pub sink: &'static str,
    pub sink_calls: usize,
    pub selected_only_png: bool,
    pub png_byte_len: usize,
    pub copied_png_byte_len: usize,
    pub effect: SelectedOutputEffectReceipt,
}

impl WgcSelectedOutputFakeSinkAcceptanceReceipt {
    pub fn proves_fake_sink_copy(&self) -> bool {
        self.diagnostic_only
            && !self.readiness_changed
            && !self.alt_a_changed
            && !self.persistent_handle_exposed
            && self.wgc_selected_png_evidence_present
            && self.fake_sink_copy_accepted
            && self.sink == "provided-fake-sink"
            && self.sink_calls == 1
            && self.selected_only_png
            && self.png_byte_len > 0
            && self.png_byte_len == self.copied_png_byte_len
            && self.effect.is_copy_only()
            && self.effect.png_byte_len == self.png_byte_len
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WgcSelectedOutputClipboardAcceptanceReceipt {
    pub source: &'static str,
    pub diagnostic_only: bool,
    pub readiness_changed: bool,
    pub alt_a_changed: bool,
    pub persistent_handle_exposed: bool,
    pub wgc_selected_png_evidence_present: bool,
    pub selected_output_effect_accepted: bool,
    pub sink: &'static str,
    pub selected_only_png: bool,
    pub png_byte_len: usize,
    pub copied_png_byte_len: usize,
    pub effect: SelectedOutputEffectReceipt,
}

impl WgcSelectedOutputClipboardAcceptanceReceipt {
    pub fn proves_clipboard_copy(&self) -> bool {
        self.diagnostic_only
            && !self.readiness_changed
            && !self.alt_a_changed
            && !self.persistent_handle_exposed
            && self.wgc_selected_png_evidence_present
            && self.selected_output_effect_accepted
            && self.selected_only_png
            && self.png_byte_len > 0
            && self.png_byte_len == self.copied_png_byte_len
            && self.effect.is_copy_only()
            && self.effect.png_byte_len == self.png_byte_len
    }
}

pub fn accept_wgc_selected_output_clipboard_with_sink(
    image: SelectedImageContract,
    explicit_opt_in: bool,
    sink_name: &'static str,
    sink: &mut impl SelectedOutputEffectSink,
) -> Result<WgcSelectedOutputClipboardAcceptanceReceipt, SelectedOutputEffectError> {
    let png_byte_len = image.byte_len();
    let selected_only_png = image.is_selected_only_png();
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
    let copied_png_byte_len = contract.image.png_bytes.len();
    let effect = perform_selected_output_effect_with_sink(
        &contract,
        SelectedOutputEffectRequest { explicit_opt_in },
        sink,
    )?;
    Ok(WgcSelectedOutputClipboardAcceptanceReceipt {
        source: "wgc-selected-png-evidence",
        diagnostic_only: true,
        readiness_changed: false,
        alt_a_changed: false,
        persistent_handle_exposed: false,
        wgc_selected_png_evidence_present: true,
        selected_output_effect_accepted: true,
        sink: sink_name,
        selected_only_png,
        png_byte_len,
        copied_png_byte_len,
        effect,
    })
}

pub fn accept_wgc_selected_output_fake_sink_copy(
    image: SelectedImageContract,
    explicit_opt_in: bool,
    sink: &mut impl SelectedOutputEffectSink,
) -> Result<WgcSelectedOutputFakeSinkAcceptanceReceipt, SelectedOutputEffectError> {
    let receipt = accept_wgc_selected_output_clipboard_with_sink(
        image,
        explicit_opt_in,
        "provided-fake-sink",
        sink,
    )?;
    Ok(WgcSelectedOutputFakeSinkAcceptanceReceipt {
        source: "wgc-selected-png-evidence",
        diagnostic_only: true,
        readiness_changed: false,
        alt_a_changed: false,
        persistent_handle_exposed: false,
        wgc_selected_png_evidence_present: true,
        fake_sink_copy_accepted: true,
        sink: "provided-fake-sink",
        sink_calls: 1,
        selected_only_png: receipt.selected_only_png,
        png_byte_len: receipt.png_byte_len,
        copied_png_byte_len: receipt.copied_png_byte_len,
        effect: receipt.effect,
    })
}

#[cfg(test)]
mod tests {
    use super::super::output::{
        ClampedSelectionRect, CropRect, ImageBounds, SelectedImageContract, SelectionRect,
    };
    use super::*;

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
        SelectedImageContract::new(
            ClampedSelectionRect {
                requested: SelectionRect::new(0, 0, 1, 1),
                crop: CropRect {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
                was_clamped: false,
            },
            vec![137, 80, 78, 71, 13, 10, 26, 10, 1],
            ImageBounds::new(1, 1),
        )
    }

    #[test]
    fn fake_sink_acceptance_proves_wgc_selected_png_can_copy() {
        let mut sink = FakeClipboardSink::default();

        let receipt = accept_wgc_selected_output_fake_sink_copy(selected_png(), true, &mut sink)
            .expect("fake sink acceptance");

        assert!(receipt.proves_fake_sink_copy());
        assert_eq!(sink.calls, 1);
        assert_eq!(sink.last_png_len, receipt.png_byte_len);
        assert!(!receipt.readiness_changed);
        assert!(!receipt.alt_a_changed);
        assert!(!receipt.persistent_handle_exposed);
    }

    #[test]
    fn fake_sink_acceptance_requires_explicit_opt_in() {
        let mut sink = FakeClipboardSink::default();

        let error = accept_wgc_selected_output_fake_sink_copy(selected_png(), false, &mut sink)
            .unwrap_err();

        assert!(matches!(
            error,
            SelectedOutputEffectError::ExplicitOptInRequired
        ));
        assert_eq!(sink.calls, 0);
    }
}

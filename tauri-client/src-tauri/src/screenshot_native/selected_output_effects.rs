use std::fmt;

use serde::{Deserialize, Serialize};

use super::selected_image_bridge::SelectedImageBridgeContract;
use super::{OutputAction, OutputBridgeTarget, OutputImageFormat};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedOutputEffectRequest {
    pub explicit_opt_in: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedOutputEffectReceipt {
    pub action: OutputAction,
    pub target: OutputBridgeTarget,
    pub format: OutputImageFormat,
    pub selected_only_png: bool,
    pub png_byte_len: usize,
    pub copied_to_clipboard: bool,
    pub save_invoked: bool,
    pub ocr_invoked: bool,
    pub translation_invoked: bool,
}

impl SelectedOutputEffectReceipt {
    pub fn is_copy_only(&self) -> bool {
        self.action == OutputAction::Copy
            && self.target == OutputBridgeTarget::Clipboard
            && self.copied_to_clipboard
            && !self.save_invoked
            && !self.ocr_invoked
            && !self.translation_invoked
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectedOutputEffectError {
    ExplicitOptInRequired,
    UnsupportedAction(OutputAction),
    InvalidSelectedPng,
    ClipboardWriteFailed(String),
}

impl fmt::Display for SelectedOutputEffectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExplicitOptInRequired => {
                formatter.write_str("selected output effects require explicit opt-in")
            }
            Self::UnsupportedAction(action) => {
                write!(
                    formatter,
                    "selected output effect only supports copy action: {action:?}"
                )
            }
            Self::InvalidSelectedPng => formatter
                .write_str("selected output effect requires valid selected-only PNG evidence"),
            Self::ClipboardWriteFailed(message) => {
                write!(
                    formatter,
                    "copy selected output to clipboard failed: {message}"
                )
            }
        }
    }
}

impl std::error::Error for SelectedOutputEffectError {}

pub trait SelectedOutputEffectSink {
    fn copy_png_to_clipboard(&mut self, png_bytes: &[u8]) -> Result<(), SelectedOutputEffectError>;
}

pub fn perform_selected_output_effect_with_sink(
    contract: &SelectedImageBridgeContract,
    request: SelectedOutputEffectRequest,
    sink: &mut impl SelectedOutputEffectSink,
) -> Result<SelectedOutputEffectReceipt, SelectedOutputEffectError> {
    if !request.explicit_opt_in {
        return Err(SelectedOutputEffectError::ExplicitOptInRequired);
    }

    if contract.action != OutputAction::Copy || contract.target != OutputBridgeTarget::Clipboard {
        return Err(SelectedOutputEffectError::UnsupportedAction(
            contract.action,
        ));
    }

    if !contract.image.is_selected_only_png() {
        return Err(SelectedOutputEffectError::InvalidSelectedPng);
    }

    sink.copy_png_to_clipboard(&contract.image.png_bytes)?;

    Ok(SelectedOutputEffectReceipt {
        action: contract.action,
        target: contract.target,
        format: contract.format,
        selected_only_png: true,
        png_byte_len: contract.image.byte_len(),
        copied_to_clipboard: true,
        save_invoked: false,
        ocr_invoked: false,
        translation_invoked: false,
    })
}

#[cfg(test)]
mod tests {
    use super::super::selected_image_bridge::build_selected_image_bridge_contract;
    use super::super::{RgbaFrame, SelectionRect};
    use super::*;

    #[derive(Default)]
    struct FakeClipboardSink {
        calls: usize,
        last_png: Vec<u8>,
        fail: Option<String>,
    }

    impl SelectedOutputEffectSink for FakeClipboardSink {
        fn copy_png_to_clipboard(
            &mut self,
            png_bytes: &[u8],
        ) -> Result<(), SelectedOutputEffectError> {
            self.calls += 1;
            self.last_png = png_bytes.to_vec();
            if let Some(message) = &self.fail {
                return Err(SelectedOutputEffectError::ClipboardWriteFailed(
                    message.clone(),
                ));
            }
            Ok(())
        }
    }

    fn copy_contract() -> SelectedImageBridgeContract {
        let frame =
            RgbaFrame::new(2, 1, vec![255, 0, 0, 255, 0, 255, 0, 255]).expect("valid test frame");
        build_selected_image_bridge_contract(
            OutputAction::Copy,
            &frame,
            SelectionRect::new(0, 0, 2, 1),
        )
        .expect("copy bridge contract")
    }

    #[test]
    fn copy_effect_requires_explicit_opt_in() {
        let contract = copy_contract();
        let mut sink = FakeClipboardSink::default();
        let error = perform_selected_output_effect_with_sink(
            &contract,
            SelectedOutputEffectRequest {
                explicit_opt_in: false,
            },
            &mut sink,
        )
        .expect_err("opt-in required");

        assert_eq!(error, SelectedOutputEffectError::ExplicitOptInRequired);
        assert_eq!(sink.calls, 0);
    }

    #[test]
    fn copy_effect_rejects_non_copy_actions_without_side_effects() {
        for action in [
            OutputAction::SaveAs,
            OutputAction::Ocr,
            OutputAction::Translate,
        ] {
            let mut contract = copy_contract();
            contract.action = action;
            contract.target = action.bridge_target().expect("bridge target");
            let mut sink = FakeClipboardSink::default();
            let error = perform_selected_output_effect_with_sink(
                &contract,
                SelectedOutputEffectRequest {
                    explicit_opt_in: true,
                },
                &mut sink,
            )
            .expect_err("unsupported action");

            assert_eq!(error, SelectedOutputEffectError::UnsupportedAction(action));
            assert_eq!(sink.calls, 0);
        }
    }

    #[test]
    fn copy_effect_writes_selected_png_once() {
        let contract = copy_contract();
        let expected_png = contract.image.png_bytes.clone();
        let mut sink = FakeClipboardSink::default();
        let receipt = perform_selected_output_effect_with_sink(
            &contract,
            SelectedOutputEffectRequest {
                explicit_opt_in: true,
            },
            &mut sink,
        )
        .expect("copy effect receipt");

        assert_eq!(sink.calls, 1);
        assert_eq!(sink.last_png, expected_png);
        assert!(receipt.is_copy_only());
        assert_eq!(receipt.png_byte_len, contract.image.byte_len());
        assert!(receipt.selected_only_png);
    }

    #[test]
    fn copy_effect_receipt_never_marks_save_ocr_translate() {
        let contract = copy_contract();
        let mut sink = FakeClipboardSink::default();
        let receipt = perform_selected_output_effect_with_sink(
            &contract,
            SelectedOutputEffectRequest {
                explicit_opt_in: true,
            },
            &mut sink,
        )
        .expect("copy effect receipt");

        assert!(!receipt.save_invoked);
        assert!(!receipt.ocr_invoked);
        assert!(!receipt.translation_invoked);
    }

    #[test]
    fn copy_effect_rejects_invalid_selected_png() {
        let mut contract = copy_contract();
        contract.image.png_bytes.clear();
        let mut sink = FakeClipboardSink::default();
        let error = perform_selected_output_effect_with_sink(
            &contract,
            SelectedOutputEffectRequest {
                explicit_opt_in: true,
            },
            &mut sink,
        )
        .expect_err("invalid png rejected");

        assert_eq!(error, SelectedOutputEffectError::InvalidSelectedPng);
        assert_eq!(sink.calls, 0);
    }

    #[test]
    fn copy_effect_propagates_clipboard_failure() {
        let contract = copy_contract();
        let mut sink = FakeClipboardSink {
            fail: Some("clipboard unavailable".to_string()),
            ..Default::default()
        };
        let error = perform_selected_output_effect_with_sink(
            &contract,
            SelectedOutputEffectRequest {
                explicit_opt_in: true,
            },
            &mut sink,
        )
        .expect_err("sink failure propagates");

        assert_eq!(sink.calls, 1);
        assert_eq!(
            error,
            SelectedOutputEffectError::ClipboardWriteFailed("clipboard unavailable".to_string())
        );
    }
}

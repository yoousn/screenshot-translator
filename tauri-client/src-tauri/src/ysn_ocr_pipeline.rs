use serde::Serialize;
use serde_json::Value;

use crate::ysn_ocr_runtime_adapter::OnnxOutputProbe;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum OcrOutputRole {
    DetectionProbabilityMap,
    ClassificationLogits,
    RecognitionLogits,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OcrDecodeOutputPlan {
    pub output_name: String,
    pub role: OcrOutputRole,
    pub shape: Vec<i64>,
    pub decoder: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OcrDecodePipelinePlan {
    pub ok: bool,
    pub model_id: String,
    pub model_type: String,
    pub decoder: String,
    pub outputs: Vec<OcrDecodeOutputPlan>,
    pub blockers: Vec<String>,
    pub runtime_inference_ready: bool,
}

pub fn build_decode_pipeline_plan(
    model: &Value,
    outputs: &[OnnxOutputProbe],
) -> OcrDecodePipelinePlan {
    let model_id = model["id"].as_str().unwrap_or("unknown").to_string();
    let model_type = model["type"].as_str().unwrap_or("unknown").to_string();
    let decoder = model["contract"]["decoder"]["type"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();
    let mut blockers = Vec::new();
    let mut output_plans = Vec::new();

    match decoder.as_str() {
        "db-text-detector" => {
            if let Some(output) = select_detection_probability_output(outputs) {
                output_plans.push(output_plan(
                    output,
                    OcrOutputRole::DetectionProbabilityMap,
                    &decoder,
                ));
            } else {
                blockers.push("DB detector output probability map was not found.".to_string());
            }
        }
        "angle-classifier" => {
            if let Some(output) = select_logits_output(outputs, 2) {
                output_plans.push(output_plan(
                    output,
                    OcrOutputRole::ClassificationLogits,
                    &decoder,
                ));
            } else {
                blockers.push("Angle classifier logits output was not found.".to_string());
            }
        }
        "ctc-text-recognizer" => {
            if let Some(output) = select_logits_output(outputs, 3) {
                output_plans.push(output_plan(
                    output,
                    OcrOutputRole::RecognitionLogits,
                    &decoder,
                ));
            } else {
                blockers.push("CTC recognition logits output was not found.".to_string());
            }
        }
        _ => blockers.push(format!("Unsupported OCR decoder contract: {decoder}")),
    }

    OcrDecodePipelinePlan {
        ok: blockers.is_empty(),
        model_id,
        model_type,
        decoder,
        outputs: output_plans,
        blockers,
        runtime_inference_ready: false,
    }
}

fn output_plan(
    output: &OnnxOutputProbe,
    role: OcrOutputRole,
    decoder: &str,
) -> OcrDecodeOutputPlan {
    OcrDecodeOutputPlan {
        output_name: output.name.clone(),
        role,
        shape: output.shape.clone(),
        decoder: decoder.to_string(),
    }
}

fn select_detection_probability_output(outputs: &[OnnxOutputProbe]) -> Option<&OnnxOutputProbe> {
    outputs.iter().find(|output| {
        has_f32_summary(output)
            && output.shape.len() >= 2
            && (contains_any(&output.name, &["prob", "sigmoid", "maps", "out"])
                || output.shape.len() == 4)
    })
}

fn select_logits_output(
    outputs: &[OnnxOutputProbe],
    expected_rank: usize,
) -> Option<&OnnxOutputProbe> {
    outputs.iter().find(|output| {
        has_f32_summary(output)
            && output.shape.len() == expected_rank
            && (contains_any(&output.name, &["logit", "prob", "softmax", "output", "out"])
                || expected_rank == 3)
    })
}

fn has_f32_summary(output: &OnnxOutputProbe) -> bool {
    output.f32_tensor.is_some()
        || output
            .element_type
            .as_deref()
            .map(|element_type| {
                element_type.to_ascii_lowercase().contains("float32")
                    || element_type.to_ascii_lowercase().contains("f32")
            })
            .unwrap_or(false)
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    let value = value.to_ascii_lowercase();
    needles.iter().any(|needle| value.contains(needle))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    fn f32_output(name: &str, shape: Vec<i64>) -> crate::ysn_ocr_runtime_adapter::OnnxOutputProbe {
        crate::ysn_ocr_runtime_adapter::OnnxOutputProbe {
            name: name.to_string(),
            element_type: Some("Float32".to_string()),
            shape: shape.clone(),
            f32_tensor: Some(crate::ysn_ocr_runtime_adapter::OnnxF32TensorSummary {
                shape: shape.iter().map(|dimension| *dimension as usize).collect(),
                element_count: 1,
                sample: vec![0.5],
                min: 0.5,
                max: 0.5,
                mean: 0.5,
            }),
        }
    }

    #[test]
    fn plans_db_detector_probability_output() {
        let model = json!({
            "id": "det-default",
            "type": "detection",
            "contract": { "decoder": { "type": "db-text-detector" } }
        });
        let plan = super::build_decode_pipeline_plan(
            &model,
            &[f32_output("sigmoid_0.tmp_0", vec![1, 1, 64, 64])],
        );
        assert!(plan.ok);
        assert_eq!(
            plan.outputs[0].role,
            super::OcrOutputRole::DetectionProbabilityMap
        );
        assert!(!plan.runtime_inference_ready);
    }

    #[test]
    fn plans_ctc_recognition_logits_output() {
        let model = json!({
            "id": "rec-latin",
            "type": "recognition",
            "contract": { "decoder": { "type": "ctc-text-recognizer" } }
        });
        let plan = super::build_decode_pipeline_plan(
            &model,
            &[f32_output("softmax_2.tmp_0", vec![1, 32, 128])],
        );
        assert!(plan.ok);
        assert_eq!(
            plan.outputs[0].role,
            super::OcrOutputRole::RecognitionLogits
        );
    }

    #[test]
    fn reports_blocker_for_missing_decoder_output() {
        let model = json!({
            "id": "rec-latin",
            "type": "recognition",
            "contract": { "decoder": { "type": "ctc-text-recognizer" } }
        });
        let plan = super::build_decode_pipeline_plan(&model, &[f32_output("bad", vec![1, 2])]);
        assert!(!plan.ok);
        assert!(plan.blockers[0].contains("CTC"));
    }
}

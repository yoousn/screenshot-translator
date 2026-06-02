use serde::Serialize;

use crate::ysn_ocr_decode::{OcrDetectionBox, OcrRecognitionLine};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RuntimeOcrBlock {
    pub text: String,
    pub confidence: f32,
    pub box_coords: Vec<Vec<i32>>,
    pub script: String,
    pub language: String,
    pub model_id: String,
}

#[derive(Debug, Clone)]
pub struct OcrPostprocessConfig {
    pub minimum_confidence: f32,
    pub default_script: String,
    pub default_language: String,
    pub model_id: String,
}

impl Default for OcrPostprocessConfig {
    fn default() -> Self {
        Self {
            minimum_confidence: 0.35,
            default_script: "unknown".to_string(),
            default_language: "auto".to_string(),
            model_id: "unknown".to_string(),
        }
    }
}

pub fn align_detections_with_recognitions(
    detections: &[OcrDetectionBox],
    recognitions: &[OcrRecognitionLine],
    config: &OcrPostprocessConfig,
) -> Vec<RuntimeOcrBlock> {
    let mut blocks: Vec<RuntimeOcrBlock> = detections
        .iter()
        .zip(recognitions.iter())
        .filter_map(|(detection, recognition)| {
            let text = recognition.text.trim().to_string();
            if text.is_empty() {
                return None;
            }
            let confidence =
                ((detection.confidence + recognition.confidence) / 2.0).clamp(0.0, 1.0);
            if confidence < config.minimum_confidence {
                return None;
            }
            Some(RuntimeOcrBlock {
                text,
                confidence,
                box_coords: detection.box_coords.clone(),
                script: config.default_script.clone(),
                language: config.default_language.clone(),
                model_id: config.model_id.clone(),
            })
        })
        .collect();
    blocks.sort_by(|a, b| {
        let ay = top(&a.box_coords);
        let by = top(&b.box_coords);
        ay.cmp(&by)
            .then(left(&a.box_coords).cmp(&left(&b.box_coords)))
    });
    blocks
}

fn top(box_coords: &[Vec<i32>]) -> i32 {
    box_coords
        .iter()
        .filter_map(|point| point.get(1))
        .min()
        .copied()
        .unwrap_or(0)
}

fn left(box_coords: &[Vec<i32>]) -> i32 {
    box_coords
        .iter()
        .filter_map(|point| point.first())
        .min()
        .copied()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use crate::ysn_ocr_decode::{OcrDetectionBox, OcrRecognitionLine};

    #[test]
    fn align_detections_with_recognitions_filters_empty_and_orders_blocks() {
        let detections = vec![
            OcrDetectionBox {
                box_coords: vec![vec![40, 20], vec![80, 20], vec![80, 40], vec![40, 40]],
                confidence: 0.9,
            },
            OcrDetectionBox {
                box_coords: vec![vec![5, 5], vec![40, 5], vec![40, 20], vec![5, 20]],
                confidence: 0.8,
            },
            OcrDetectionBox {
                box_coords: vec![vec![5, 50], vec![40, 50], vec![40, 70], vec![5, 70]],
                confidence: 0.1,
            },
        ];
        let recognitions = vec![
            OcrRecognitionLine {
                text: "World".to_string(),
                confidence: 0.9,
                token_count: 5,
            },
            OcrRecognitionLine {
                text: "Hello".to_string(),
                confidence: 0.9,
                token_count: 5,
            },
            OcrRecognitionLine {
                text: "low".to_string(),
                confidence: 0.1,
                token_count: 3,
            },
        ];
        let config = super::OcrPostprocessConfig {
            minimum_confidence: 0.35,
            default_script: "latin".to_string(),
            default_language: "en".to_string(),
            model_id: "rec-latin".to_string(),
        };
        let blocks = super::align_detections_with_recognitions(&detections, &recognitions, &config);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].text, "Hello");
        assert_eq!(blocks[1].text, "World");
        assert_eq!(blocks[0].model_id, "rec-latin");
    }
}

use serde::Serialize;
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OcrDetectionBox {
    pub box_coords: Vec<Vec<i32>>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OcrRecognitionLine {
    pub text: String,
    pub confidence: f32,
    pub token_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OcrCtcLogitsBridgePlan {
    pub ok: bool,
    pub output_name: String,
    pub shape: Vec<i64>,
    pub time_steps: usize,
    pub class_count: usize,
    pub dictionary_size: usize,
    pub blank_token_id: usize,
    pub blockers: Vec<String>,
    pub runtime_inference_ready: bool,
}

#[derive(Debug, Clone)]
pub struct DbTextDetectorConfig {
    pub probability_threshold: f32,
    pub minimum_area: usize,
    pub original_width: u32,
    pub original_height: u32,
}

impl Default for DbTextDetectorConfig {
    fn default() -> Self {
        Self {
            probability_threshold: 0.3,
            minimum_area: 3,
            original_width: 0,
            original_height: 0,
        }
    }
}

pub fn decode_db_probability_map(
    probabilities: &[f32],
    width: usize,
    height: usize,
    config: &DbTextDetectorConfig,
) -> Result<Vec<OcrDetectionBox>, String> {
    if width == 0 || height == 0 {
        return Err("DB detector probability map dimensions must be non-zero.".to_string());
    }
    if probabilities.len() != width * height {
        return Err(format!(
            "DB detector probability map length mismatch: expected {}, got {}",
            width * height,
            probabilities.len()
        ));
    }
    let mut visited = vec![false; probabilities.len()];
    let mut boxes = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let index = y * width + x;
            if visited[index] || probabilities[index] < config.probability_threshold {
                continue;
            }
            let component = collect_component(
                probabilities,
                width,
                height,
                index,
                config.probability_threshold,
                &mut visited,
            );
            if component.len() < config.minimum_area {
                continue;
            }
            boxes.push(component_to_box(
                probabilities,
                &component,
                width,
                height,
                config,
            ));
        }
    }
    boxes.sort_by(|a, b| {
        let ay = a.box_coords[0][1];
        let by = b.box_coords[0][1];
        ay.cmp(&by)
            .then(a.box_coords[0][0].cmp(&b.box_coords[0][0]))
    });
    Ok(boxes)
}

fn collect_component(
    probabilities: &[f32],
    width: usize,
    height: usize,
    start: usize,
    threshold: f32,
    visited: &mut [bool],
) -> Vec<usize> {
    let mut queue = VecDeque::new();
    let mut component = Vec::new();
    queue.push_back(start);
    visited[start] = true;
    while let Some(index) = queue.pop_front() {
        component.push(index);
        let x = index % width;
        let y = index / width;
        let neighbors = [
            (x.checked_sub(1), Some(y)),
            (x.checked_add(1).filter(|next| *next < width), Some(y)),
            (Some(x), y.checked_sub(1)),
            (Some(x), y.checked_add(1).filter(|next| *next < height)),
        ];
        for (nx, ny) in neighbors {
            let (Some(nx), Some(ny)) = (nx, ny) else {
                continue;
            };
            let neighbor_index = ny * width + nx;
            if !visited[neighbor_index] && probabilities[neighbor_index] >= threshold {
                visited[neighbor_index] = true;
                queue.push_back(neighbor_index);
            }
        }
    }
    component
}

fn component_to_box(
    probabilities: &[f32],
    component: &[usize],
    width: usize,
    height: usize,
    config: &DbTextDetectorConfig,
) -> OcrDetectionBox {
    let min_x = component
        .iter()
        .map(|index| index % width)
        .min()
        .unwrap_or(0);
    let max_x = component
        .iter()
        .map(|index| index % width)
        .max()
        .unwrap_or(min_x)
        + 1;
    let min_y = component
        .iter()
        .map(|index| index / width)
        .min()
        .unwrap_or(0);
    let max_y = component
        .iter()
        .map(|index| index / width)
        .max()
        .unwrap_or(min_y)
        + 1;
    let confidence = component
        .iter()
        .map(|index| probabilities[*index])
        .sum::<f32>()
        / component.len() as f32;
    let original_width = if config.original_width == 0 {
        width as u32
    } else {
        config.original_width
    };
    let original_height = if config.original_height == 0 {
        height as u32
    } else {
        config.original_height
    };
    let scale_x = original_width as f32 / width as f32;
    let scale_y = original_height as f32 / height as f32;
    OcrDetectionBox {
        box_coords: vec![
            vec![
                (min_x as f32 * scale_x).round() as i32,
                (min_y as f32 * scale_y).round() as i32,
            ],
            vec![
                (max_x as f32 * scale_x).round() as i32,
                (min_y as f32 * scale_y).round() as i32,
            ],
            vec![
                (max_x as f32 * scale_x).round() as i32,
                (max_y as f32 * scale_y).round() as i32,
            ],
            vec![
                (min_x as f32 * scale_x).round() as i32,
                (max_y as f32 * scale_y).round() as i32,
            ],
        ],
        confidence,
    }
}

pub fn decode_ctc_tokens(
    token_ids: &[usize],
    token_probabilities: &[f32],
    dictionary: &[String],
    blank_token_id: usize,
) -> Result<OcrRecognitionLine, String> {
    if token_ids.len() != token_probabilities.len() {
        return Err("CTC token id/probability length mismatch.".to_string());
    }
    let mut text = String::new();
    let mut last_token: Option<usize> = None;
    let mut confidences = Vec::new();
    for (&token_id, &probability) in token_ids.iter().zip(token_probabilities.iter()) {
        if token_id == blank_token_id {
            last_token = None;
            continue;
        }
        if last_token == Some(token_id) {
            continue;
        }
        let token = dictionary
            .get(token_id)
            .ok_or_else(|| format!("CTC token id is outside dictionary: {token_id}"))?;
        text.push_str(token);
        confidences.push(probability.clamp(0.0, 1.0));
        last_token = Some(token_id);
    }
    let confidence = if confidences.is_empty() {
        0.0
    } else {
        confidences.iter().sum::<f32>() / confidences.len() as f32
    };
    Ok(OcrRecognitionLine {
        text,
        confidence,
        token_count: confidences.len(),
    })
}

pub fn build_ctc_logits_bridge_plan(
    output: &crate::ysn_ocr_runtime_adapter::OnnxOutputProbe,
    dictionary: &[String],
    blank_token_id: usize,
) -> OcrCtcLogitsBridgePlan {
    let mut blockers = Vec::new();
    if !output
        .element_type
        .as_deref()
        .map(|element_type| element_type.eq_ignore_ascii_case("Float32"))
        .unwrap_or(false)
    {
        blockers.push(format!(
            "CTC logits output {} must be Float32.",
            output.name
        ));
    }
    if output.shape.len() != 3 {
        blockers.push(format!(
            "CTC logits output {} must have rank 3 [batch,time,class].",
            output.name
        ));
    }
    let batch = output.shape.first().copied().unwrap_or(0);
    if batch != 1 {
        blockers.push(format!(
            "CTC logits output {} currently supports batch 1, got {}.",
            output.name, batch
        ));
    }
    let time_steps = output
        .shape
        .get(1)
        .copied()
        .filter(|value| *value > 0)
        .unwrap_or(0) as usize;
    let class_count = output
        .shape
        .get(2)
        .copied()
        .filter(|value| *value > 0)
        .unwrap_or(0) as usize;
    if time_steps == 0 {
        blockers.push("CTC logits time dimension must be positive.".to_string());
    }
    if class_count == 0 {
        blockers.push("CTC logits class dimension must be positive.".to_string());
    }
    if class_count > dictionary.len() {
        blockers.push(format!(
            "CTC logits class count {} exceeds dictionary size {}.",
            class_count,
            dictionary.len()
        ));
    }
    if blank_token_id >= dictionary.len() {
        blockers.push(format!(
            "CTC blank token id {} is outside dictionary size {}.",
            blank_token_id,
            dictionary.len()
        ));
    }
    OcrCtcLogitsBridgePlan {
        ok: blockers.is_empty(),
        output_name: output.name.clone(),
        shape: output.shape.clone(),
        time_steps,
        class_count,
        dictionary_size: dictionary.len(),
        blank_token_id,
        blockers,
        runtime_inference_ready: false,
    }
}

pub fn decode_ctc_logits(
    logits: &[f32],
    time_steps: usize,
    class_count: usize,
    dictionary: &[String],
    blank_token_id: usize,
) -> Result<OcrRecognitionLine, String> {
    if time_steps == 0 || class_count == 0 {
        return Err("CTC logits dimensions must be positive.".to_string());
    }
    let expected_len = time_steps
        .checked_mul(class_count)
        .ok_or_else(|| "CTC logits dimensions are too large.".to_string())?;
    if logits.len() != expected_len {
        return Err(format!(
            "CTC logits length mismatch: expected {expected_len}, got {}.",
            logits.len()
        ));
    }
    if class_count > dictionary.len() {
        return Err(format!(
            "CTC logits class count {class_count} exceeds dictionary size {}.",
            dictionary.len()
        ));
    }
    let mut token_ids = Vec::with_capacity(time_steps);
    let mut token_probabilities = Vec::with_capacity(time_steps);
    for step in 0..time_steps {
        let start = step * class_count;
        let row = &logits[start..start + class_count];
        let (token_id, probability) = row
            .iter()
            .copied()
            .enumerate()
            .max_by(|(_, left), (_, right)| left.total_cmp(right))
            .ok_or_else(|| "CTC logits row is empty.".to_string())?;
        token_ids.push(token_id);
        token_probabilities.push(probability.clamp(0.0, 1.0));
    }
    decode_ctc_tokens(&token_ids, &token_probabilities, dictionary, blank_token_id)
}

#[cfg(test)]
mod tests {
    #[test]
    fn decode_db_probability_map_extracts_scaled_boxes_in_reading_order() {
        let probabilities = vec![
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.8, 0.9, 0.0, 0.0, 0.0, 0.7, 0.8, 0.0, 0.0, 0.0, 0.0,
            0.0, 0.6, 0.7, 0.0, 0.0, 0.0, 0.6, 0.8,
        ];
        let config = super::DbTextDetectorConfig {
            probability_threshold: 0.5,
            minimum_area: 2,
            original_width: 50,
            original_height: 50,
        };
        let boxes = super::decode_db_probability_map(&probabilities, 5, 5, &config).unwrap();
        assert_eq!(boxes.len(), 2);
        assert_eq!(boxes[0].box_coords[0], vec![10, 10]);
        assert_eq!(boxes[0].box_coords[2], vec![30, 30]);
        assert_eq!(boxes[1].box_coords[0], vec![30, 30]);
        assert!(boxes[0].confidence > boxes[1].confidence);
    }

    #[test]
    fn decode_ctc_tokens_collapses_repeats_and_preserves_words() {
        let dictionary = [
            "".to_string(),
            "A".to_string(),
            "d".to_string(),
            " ".to_string(),
            "P".to_string(),
            "ATH".to_string(),
        ];
        let decoded = super::decode_ctc_tokens(
            &[1, 1, 0, 2, 2, 3, 4, 5],
            &[0.9, 0.8, 0.2, 0.7, 0.7, 0.95, 0.9, 0.85],
            &dictionary,
            0,
        )
        .unwrap();
        assert_eq!(decoded.text, "Ad PATH");
        assert_eq!(decoded.token_count, 5);
        assert!(decoded.confidence > 0.8);
    }

    fn output_probe(shape: Vec<i64>) -> crate::ysn_ocr_runtime_adapter::OnnxOutputProbe {
        crate::ysn_ocr_runtime_adapter::OnnxOutputProbe {
            name: "softmax_2.tmp_0".to_string(),
            element_type: Some("Float32".to_string()),
            shape,
            f32_tensor: None,
        }
    }

    #[test]
    fn build_ctc_logits_bridge_plan_accepts_rank3_logits() {
        let dictionary = vec!["".to_string(), "A".to_string(), "B".to_string()];
        let plan =
            super::build_ctc_logits_bridge_plan(&output_probe(vec![1, 8, 3]), &dictionary, 0);

        assert!(plan.ok);
        assert_eq!(plan.time_steps, 8);
        assert_eq!(plan.class_count, 3);
        assert!(!plan.runtime_inference_ready);
    }

    #[test]
    fn build_ctc_logits_bridge_plan_reports_dictionary_mismatch() {
        let dictionary = vec!["".to_string(), "A".to_string()];
        let plan =
            super::build_ctc_logits_bridge_plan(&output_probe(vec![1, 8, 3]), &dictionary, 0);

        assert!(!plan.ok);
        assert!(plan
            .blockers
            .iter()
            .any(|blocker| blocker.contains("dictionary size")));
    }

    #[test]
    fn decode_ctc_logits_argmaxes_and_collapses_tokens() {
        let dictionary = vec!["".to_string(), "A".to_string(), "B".to_string()];
        let decoded = super::decode_ctc_logits(
            &[0.1, 0.9, 0.2, 0.1, 0.8, 0.3, 0.7, 0.2, 0.1, 0.1, 0.2, 0.95],
            4,
            3,
            &dictionary,
            0,
        )
        .unwrap();

        assert_eq!(decoded.text, "AB");
        assert_eq!(decoded.token_count, 2);
        assert!(decoded.confidence > 0.9);
    }

    #[test]
    fn decode_ctc_logits_rejects_length_mismatch() {
        let dictionary = vec!["".to_string(), "A".to_string()];
        let error = super::decode_ctc_logits(&[0.1, 0.9, 0.1], 2, 2, &dictionary, 0).unwrap_err();

        assert!(error.contains("length mismatch"));
    }
}

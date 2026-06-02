use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

static ONNX_SESSION_CACHE: OnceLock<Mutex<HashMap<PathBuf, ort::session::Session>>> =
    OnceLock::new();

fn cached_sessions() -> &'static Mutex<HashMap<PathBuf, ort::session::Session>> {
    ONNX_SESSION_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn with_cached_session<T>(
    model_path: &Path,
    run: impl FnOnce(&mut ort::session::Session) -> Result<T, String>,
) -> Result<T, String> {
    let canonical_path = model_path.canonicalize().map_err(|error| {
        format!(
            "failed to resolve ONNX model path {}: {error}",
            model_path.display()
        )
    })?;
    let mut sessions = cached_sessions()
        .lock()
        .map_err(|_| "ONNX session cache lock poisoned".to_string())?;
    if !sessions.contains_key(&canonical_path) {
        let session = ort::session::Session::builder()
            .map_err(|error| format!("failed to create ONNX session builder: {error}"))?
            .commit_from_file(&canonical_path)
            .map_err(|error| format!("failed to load ONNX model session: {error}"))?;
        sessions.insert(canonical_path.clone(), session);
    }
    let session = sessions
        .get_mut(&canonical_path)
        .ok_or_else(|| "ONNX session cache did not return loaded session".to_string())?;
    run(session)
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OnnxIoProbe {
    pub name: String,
    pub kind: String,
    pub is_tensor: bool,
    pub element_type: Option<String>,
    pub shape: Vec<i64>,
    pub dynamic_dimensions: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OnnxInputBindingPlan {
    pub ok: bool,
    pub input_name: Option<String>,
    pub model_shape: Vec<i64>,
    pub tensor_shape: Vec<usize>,
    pub element_type: Option<String>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct OnnxOutputProbe {
    pub name: String,
    pub element_type: Option<String>,
    pub shape: Vec<i64>,
    pub f32_tensor: Option<OnnxF32TensorSummary>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OnnxF32TensorSummary {
    pub shape: Vec<usize>,
    pub element_count: usize,
    pub sample: Vec<f32>,
    pub min: f32,
    pub max: f32,
    pub mean: f32,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct OnnxF32TensorOutput {
    pub name: String,
    pub shape: Vec<usize>,
    pub data: Vec<f32>,
}
#[derive(Debug, Serialize)]
pub struct OnnxInferenceProbe {
    pub ok: bool,
    pub model_path: String,
    pub input_name: String,
    pub input_shape: Vec<usize>,
    pub output_count: usize,
    pub outputs: Vec<OnnxOutputProbe>,
    pub elapsed_ms: u128,
    pub status: String,
}

pub fn summarize_f32_tensor(
    shape: &[usize],
    data: &[f32],
    sample_limit: usize,
) -> Result<OnnxF32TensorSummary, String> {
    let expected_len: usize = shape.iter().product();
    if expected_len != data.len() {
        return Err(format!(
            "ONNX f32 output tensor length mismatch: expected {expected_len}, got {}",
            data.len()
        ));
    }
    if data.is_empty() {
        return Ok(OnnxF32TensorSummary {
            shape: shape.to_vec(),
            element_count: 0,
            sample: Vec::new(),
            min: 0.0,
            max: 0.0,
            mean: 0.0,
        });
    }
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    let mut sum = 0.0;
    for value in data {
        min = min.min(*value);
        max = max.max(*value);
        sum += *value;
    }
    Ok(OnnxF32TensorSummary {
        shape: shape.to_vec(),
        element_count: data.len(),
        sample: data.iter().take(sample_limit).copied().collect(),
        min,
        max,
        mean: sum / data.len() as f32,
    })
}
#[derive(Debug, Serialize)]
pub struct OnnxSessionProbe {
    pub ok: bool,
    pub model_path: String,
    pub input_count: usize,
    pub output_count: usize,
    pub inputs: Vec<OnnxIoProbe>,
    pub outputs: Vec<OnnxIoProbe>,
    pub inference_status: String,
}

fn outlet_probe(kind: &str, outlet: &ort::value::Outlet) -> OnnxIoProbe {
    let dtype = outlet.dtype();
    let shape: Vec<i64> = dtype
        .tensor_shape()
        .map(|shape| shape.iter().copied().collect())
        .unwrap_or_default();
    OnnxIoProbe {
        name: outlet.name().to_string(),
        kind: kind.to_string(),
        is_tensor: dtype.is_tensor(),
        element_type: dtype
            .tensor_type()
            .map(|element_type| format!("{element_type:?}")),
        dynamic_dimensions: shape.iter().any(|dimension| *dimension < 0),
        shape,
    }
}

pub fn probe_onnx_session(model_path: &Path) -> Result<OnnxSessionProbe, String> {
    if !model_path.exists() {
        return Err(format!(
            "ONNX model file does not exist: {}",
            model_path.to_string_lossy()
        ));
    }
    let session = ort::session::Session::builder()
        .map_err(|error| format!("failed to create ONNX session builder: {error}"))?
        .commit_from_file(model_path)
        .map_err(|error| format!("failed to load ONNX model session: {error}"))?;

    let inputs: Vec<OnnxIoProbe> = session
        .inputs()
        .iter()
        .map(|input| outlet_probe("input", input))
        .collect();
    let outputs: Vec<OnnxIoProbe> = session
        .outputs()
        .iter()
        .map(|output| outlet_probe("output", output))
        .collect();

    Ok(OnnxSessionProbe {
        ok: true,
        model_path: model_path.to_string_lossy().to_string(),
        input_count: inputs.len(),
        output_count: outputs.len(),
        inputs,
        outputs,
        inference_status:
            "metadata-ready; tensor preprocessing and execution are not wired to OCR pipeline yet"
                .to_string(),
    })
}

pub fn probe_onnx_session_readiness(model_path: &Path) -> Value {
    let model_path_text = model_path.to_string_lossy().to_string();
    if !model_path.exists() {
        return json!({
            "ok": false,
            "runtimeInferenceReady": false,
            "status": "model-file-missing",
            "modelPath": model_path_text,
            "blockers": [format!("ONNX model file does not exist: {model_path_text}")],
            "nextAction": "install-or-repair-model-pack",
            "sessionProbe": null
        });
    }
    if !model_path.is_file() {
        return json!({
            "ok": false,
            "runtimeInferenceReady": false,
            "status": "model-path-not-file",
            "modelPath": model_path_text,
            "blockers": [format!("ONNX model path is not a file: {model_path_text}")],
            "nextAction": "repair-active-model-files",
            "sessionProbe": null
        });
    }

    match probe_onnx_session(model_path) {
        Ok(probe) => json!({
            "ok": true,
            "runtimeInferenceReady": false,
            "status": "session-metadata-ready",
            "modelPath": model_path_text,
            "blockers": [],
            "nextAction": "wire-real-onnx-inference-and-self-test",
            "sessionProbe": probe,
            "message": "ONNX session metadata loaded. Runtime readiness remains false until inference, decode, postprocess, and self-test pass."
        }),
        Err(error) => json!({
            "ok": false,
            "runtimeInferenceReady": false,
            "status": "session-load-failed",
            "modelPath": model_path_text,
            "blockers": [error],
            "nextAction": "repair-active-model-files",
            "sessionProbe": null
        }),
    }
}

fn shape_to_usize(shape: &[i64]) -> Result<Vec<usize>, String> {
    shape
        .iter()
        .map(|dimension| {
            if *dimension < 0 {
                Err(format!(
                    "ONNX f32 output tensor has dynamic dimension in runtime output: {dimension}"
                ))
            } else {
                Ok(*dimension as usize)
            }
        })
        .collect()
}

fn extract_f32_tensor_summary(value: &ort::value::DynValue) -> Option<OnnxF32TensorSummary> {
    let Ok((shape, data)) = value.try_extract_tensor::<f32>() else {
        return None;
    };
    let shape = shape_to_usize(shape).ok()?;
    summarize_f32_tensor(&shape, data, 16).ok()
}

pub fn build_nchw_f32_input_binding_plan(
    inputs: &[OnnxIoProbe],
    tensor: &crate::ysn_ocr_preprocess::OcrTensorInput,
) -> OnnxInputBindingPlan {
    let mut blockers = Vec::new();
    if let Err(error) = validate_ocr_tensor_scaffold(tensor) {
        blockers.push(error);
    }
    let Some(input) = inputs.first() else {
        return OnnxInputBindingPlan {
            ok: false,
            input_name: None,
            model_shape: Vec::new(),
            tensor_shape: tensor.shape.clone(),
            element_type: None,
            blockers: vec!["ONNX model has no inputs.".to_string()],
        };
    };
    if !input.is_tensor {
        blockers.push(format!("ONNX input {} is not a tensor.", input.name));
    }
    if input.element_type.as_deref() != Some("Float32") {
        blockers.push(format!(
            "ONNX input {} expects {}, but OCR scaffold prepared f32.",
            input.name,
            input.element_type.as_deref().unwrap_or("unknown")
        ));
    }
    if !input.shape.is_empty() {
        if input.shape.len() != tensor.shape.len() {
            blockers.push(format!(
                "ONNX input {} rank mismatch: model rank {}, tensor rank {}.",
                input.name,
                input.shape.len(),
                tensor.shape.len()
            ));
        } else {
            for (index, (model_dim, tensor_dim)) in
                input.shape.iter().zip(tensor.shape.iter()).enumerate()
            {
                if *model_dim >= 0 && (*model_dim as usize) != *tensor_dim {
                    blockers.push(format!(
                        "ONNX input {} dimension {} mismatch: model {}, tensor {}.",
                        input.name, index, model_dim, tensor_dim
                    ));
                }
            }
        }
    }
    OnnxInputBindingPlan {
        ok: blockers.is_empty(),
        input_name: Some(input.name.clone()),
        model_shape: input.shape.clone(),
        tensor_shape: tensor.shape.clone(),
        element_type: input.element_type.clone(),
        blockers,
    }
}

fn validate_tensor_against_input(
    input: &ort::value::Outlet,
    tensor: &crate::ysn_ocr_preprocess::OcrTensorInput,
) -> Result<(), String> {
    let dtype = input.dtype();
    let Some(element_type) = dtype.tensor_type() else {
        return Err(format!("ONNX input {} is not a tensor.", input.name()));
    };
    if format!("{element_type:?}") != "Float32" {
        return Err(format!(
            "ONNX input {} expects {element_type:?}, but OCR scaffold prepared f32.",
            input.name()
        ));
    }
    if let Some(shape) = dtype.tensor_shape() {
        let dims: Vec<i64> = shape.iter().copied().collect();
        if dims.len() != tensor.shape.len() {
            return Err(format!(
                "ONNX input {} rank mismatch: model rank {}, tensor rank {}.",
                input.name(),
                dims.len(),
                tensor.shape.len()
            ));
        }
        for (index, (model_dim, tensor_dim)) in dims.iter().zip(tensor.shape.iter()).enumerate() {
            if *model_dim >= 0 && (*model_dim as usize) != *tensor_dim {
                return Err(format!(
                    "ONNX input {} dimension {} mismatch: model {}, tensor {}.",
                    input.name(),
                    index,
                    model_dim,
                    tensor_dim
                ));
            }
        }
    }
    Ok(())
}

fn validate_ocr_tensor_scaffold(
    tensor: &crate::ysn_ocr_preprocess::OcrTensorInput,
) -> Result<(), String> {
    if tensor.shape.len() != 4
        || tensor.channels != 3
        || tensor.layout != "NCHW"
        || tensor.element_type != "f32"
    {
        return Err("OCR inference scaffold expects a 4D NCHW RGB f32 tensor.".to_string());
    }
    let expected_len: usize = tensor.shape.iter().product();
    if expected_len != tensor.data.len() {
        return Err(format!(
            "OCR tensor data length mismatch: expected {expected_len}, got {}",
            tensor.data.len()
        ));
    }
    Ok(())
}

pub fn prewarm_onnx_session(model_path: &Path) -> Result<(), String> {
    with_cached_session(model_path, |_| Ok(()))
}

pub fn run_onnx_nchw_f32_outputs(
    model_path: &Path,
    tensor: &crate::ysn_ocr_preprocess::OcrTensorInput,
) -> Result<Vec<OnnxF32TensorOutput>, String> {
    if !model_path.exists() {
        return Err(format!(
            "ONNX model file does not exist: {}",
            model_path.to_string_lossy()
        ));
    }
    validate_ocr_tensor_scaffold(tensor)?;

    with_cached_session(model_path, |session| {
        let first_input = session
            .inputs()
            .first()
            .ok_or_else(|| "ONNX model has no inputs.".to_string())?;
        let binding_plan =
            build_nchw_f32_input_binding_plan(&[outlet_probe("input", first_input)], tensor);
        if !binding_plan.ok {
            return Err(format!(
                "ONNX input binding plan is blocked: {}",
                binding_plan.blockers.join("; ")
            ));
        }
        validate_tensor_against_input(first_input, tensor)?;
        let input_name = first_input.name().to_string();
        let array =
            ndarray::Array::from_shape_vec(ndarray::IxDyn(&tensor.shape), tensor.data.clone())
                .map_err(|error| format!("failed to create OCR input tensor: {error}"))?;
        let input_tensor = ort::value::Tensor::from_array(array)
            .map_err(|error| format!("failed to create ONNX input tensor: {error}"))?;
        let outputs = session
            .run(ort::inputs![input_name.as_str() => input_tensor])
            .map_err(|error| format!("ONNX inference failed: {error}"))?;

        let mut tensors = Vec::new();
        for name in outputs
            .keys()
            .map(|key| key.to_string())
            .collect::<Vec<_>>()
        {
            let Some(value) = outputs.get(&name) else {
                continue;
            };
            let (shape, data) = value
                .try_extract_tensor::<f32>()
                .map_err(|error| format!("failed to extract ONNX f32 output {name}: {error}"))?;
            let shape = shape_to_usize(shape)?;
            let expected_len: usize = shape.iter().product();
            if expected_len != data.len() {
                return Err(format!(
                    "ONNX f32 output tensor length mismatch for {name}: expected {expected_len}, got {}",
                    data.len()
                ));
            }
            tensors.push(OnnxF32TensorOutput {
                name,
                shape,
                data: data.to_vec(),
            });
        }
        Ok(tensors)
    })
}
pub fn run_onnx_nchw_f32_probe(
    model_path: &Path,
    tensor: &crate::ysn_ocr_preprocess::OcrTensorInput,
) -> Result<OnnxInferenceProbe, String> {
    if !model_path.exists() {
        return Err(format!(
            "ONNX model file does not exist: {}",
            model_path.to_string_lossy()
        ));
    }
    validate_ocr_tensor_scaffold(tensor)?;

    let mut session = ort::session::Session::builder()
        .map_err(|error| format!("failed to create ONNX session builder: {error}"))?
        .commit_from_file(model_path)
        .map_err(|error| format!("failed to load ONNX model session: {error}"))?;
    let first_input = session
        .inputs()
        .first()
        .ok_or_else(|| "ONNX model has no inputs.".to_string())?;
    let binding_plan =
        build_nchw_f32_input_binding_plan(&[outlet_probe("input", first_input)], tensor);
    if !binding_plan.ok {
        return Err(format!(
            "ONNX input binding plan is blocked: {}",
            binding_plan.blockers.join("; ")
        ));
    }
    validate_tensor_against_input(first_input, tensor)?;
    let input_name = first_input.name().to_string();
    let array = ndarray::Array::from_shape_vec(ndarray::IxDyn(&tensor.shape), tensor.data.clone())
        .map_err(|error| format!("failed to create OCR input tensor: {error}"))?;
    let input_tensor = ort::value::Tensor::from_array(array)
        .map_err(|error| format!("failed to create ONNX input tensor: {error}"))?;

    let started = Instant::now();
    let outputs = session
        .run(ort::inputs![input_name.as_str() => input_tensor])
        .map_err(|error| format!("ONNX inference failed: {error}"))?;
    let output_names: Vec<String> = outputs.keys().map(|key| key.to_string()).collect();
    let mut output_probes = Vec::new();
    for name in output_names {
        if let Some(value) = outputs.get(&name) {
            let dtype = value.dtype();
            let shape: Vec<i64> = dtype
                .tensor_shape()
                .map(|shape| shape.iter().copied().collect())
                .unwrap_or_default();
            let f32_tensor = extract_f32_tensor_summary(value);
            output_probes.push(OnnxOutputProbe {
                name,
                element_type: dtype
                    .tensor_type()
                    .map(|element_type| format!("{element_type:?}")),
                shape,
                f32_tensor,
            });
        }
    }

    Ok(OnnxInferenceProbe {
        ok: true,
        model_path: model_path.to_string_lossy().to_string(),
        input_name,
        input_shape: tensor.shape.clone(),
        output_count: output_probes.len(),
        outputs: output_probes,
        elapsed_ms: started.elapsed().as_millis(),
        status: "inference-executed; OCR postprocessing is not wired yet".to_string(),
    })
}

#[cfg(test)]
mod tests {
    fn input_probe(shape: Vec<i64>, element_type: Option<&str>) -> super::OnnxIoProbe {
        super::OnnxIoProbe {
            name: "x".to_string(),
            kind: "input".to_string(),
            is_tensor: true,
            element_type: element_type.map(|value| value.to_string()),
            shape,
            dynamic_dimensions: false,
        }
    }

    fn tensor() -> crate::ysn_ocr_preprocess::OcrTensorInput {
        crate::ysn_ocr_preprocess::OcrTensorInput {
            shape: vec![1, 3, 2, 2],
            width: 2,
            height: 2,
            original_width: 2,
            original_height: 2,
            channels: 3,
            layout: "NCHW".to_string(),
            element_type: "f32".to_string(),
            data: vec![0.0; 12],
        }
    }

    #[test]
    fn test_probe_onnx_session_readiness_reports_missing_model() {
        let path = std::env::temp_dir().join("ysn-ocr-missing-session-probe.onnx");
        let report = super::probe_onnx_session_readiness(&path);

        assert_eq!(report["ok"].as_bool(), Some(false));
        assert_eq!(report["runtimeInferenceReady"].as_bool(), Some(false));
        assert_eq!(report["status"].as_str(), Some("model-file-missing"));
        assert!(report["blockers"][0]
            .as_str()
            .unwrap()
            .contains("does not exist"));
    }

    #[test]
    fn test_probe_onnx_session_readiness_reports_damaged_model() {
        let root = std::env::temp_dir().join(format!(
            "ysn-ocr-damaged-session-{}",
            chrono::Local::now()
                .timestamp_nanos_opt()
                .unwrap_or_default()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("damaged.onnx");
        std::fs::write(&path, b"not-an-onnx-model").unwrap();

        let report = super::probe_onnx_session_readiness(&path);

        assert_eq!(report["ok"].as_bool(), Some(false));
        assert_eq!(report["runtimeInferenceReady"].as_bool(), Some(false));
        assert_eq!(report["status"].as_str(), Some("session-load-failed"));
        assert!(report["blockers"].as_array().unwrap().len() == 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_validate_ocr_tensor_scaffold_accepts_valid_tensor() {
        assert!(super::validate_ocr_tensor_scaffold(&tensor()).is_ok());
    }

    #[test]
    fn test_build_input_binding_plan_accepts_dynamic_batch_and_width() {
        let plan = super::build_nchw_f32_input_binding_plan(
            &[input_probe(vec![-1, 3, 2, -1], Some("Float32"))],
            &tensor(),
        );

        assert!(plan.ok);
        assert_eq!(plan.input_name, Some("x".to_string()));
        assert!(plan.blockers.is_empty());
    }

    #[test]
    fn test_build_input_binding_plan_reports_dtype_and_rank_blockers() {
        let plan = super::build_nchw_f32_input_binding_plan(
            &[input_probe(vec![1, 3, 2], Some("Uint8"))],
            &tensor(),
        );

        assert!(!plan.ok);
        assert!(plan
            .blockers
            .iter()
            .any(|blocker| blocker.contains("Uint8")));
        assert!(plan
            .blockers
            .iter()
            .any(|blocker| blocker.contains("rank mismatch")));
    }

    #[test]
    fn test_build_input_binding_plan_reports_static_dimension_mismatch() {
        let plan = super::build_nchw_f32_input_binding_plan(
            &[input_probe(vec![1, 3, 4, 2], Some("Float32"))],
            &tensor(),
        );

        assert!(!plan.ok);
        assert!(plan
            .blockers
            .iter()
            .any(|blocker| blocker.contains("dimension 2")));
    }

    #[test]
    fn test_build_input_binding_plan_reports_missing_input() {
        let plan = super::build_nchw_f32_input_binding_plan(&[], &tensor());

        assert!(!plan.ok);
        assert_eq!(plan.input_name, None);
        assert!(plan.blockers[0].contains("no inputs"));
    }

    #[test]
    fn test_validate_ocr_tensor_scaffold_rejects_bad_layout() {
        let mut tensor = tensor();
        tensor.layout = "NHWC".to_string();
        assert!(super::validate_ocr_tensor_scaffold(&tensor)
            .unwrap_err()
            .contains("4D NCHW RGB f32"));
    }

    #[test]
    fn test_validate_ocr_tensor_scaffold_rejects_length_mismatch() {
        let mut tensor = tensor();
        tensor.data.pop();
        assert!(super::validate_ocr_tensor_scaffold(&tensor)
            .unwrap_err()
            .contains("data length mismatch"));
    }
    #[test]
    fn test_summarize_f32_tensor_reports_stats() {
        let summary =
            super::summarize_f32_tensor(&[1, 2, 3], &[0.1, 0.5, 0.9, 1.0, -0.5, 0.0], 3).unwrap();
        assert_eq!(summary.shape, vec![1, 2, 3]);
        assert_eq!(summary.element_count, 6);
        assert_eq!(summary.sample, vec![0.1, 0.5, 0.9]);
        assert_eq!(summary.min, -0.5);
        assert_eq!(summary.max, 1.0);
        assert!((summary.mean - 0.33333334).abs() < 0.0001);
    }

    #[test]
    fn test_shape_to_usize_rejects_dynamic_runtime_output() {
        let error = super::shape_to_usize(&[1, -1, 32]).unwrap_err();
        assert!(error.contains("dynamic dimension"));
    }
}

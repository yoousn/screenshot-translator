use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};

pub(crate) const RUNTIME_VERSION: &str = "0.1.0-planned";
pub(crate) const MODEL_SET_VERSION: &str = "2026.06.ocr.v1";

pub(crate) fn model_root(app: &AppHandle) -> Result<PathBuf, String> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data dir: {e}"))?;
    Ok(base.join("models").join("ocr"))
}

fn default_model_index() -> Value {
    crate::ysn_ocr_model_index::default_model_index(MODEL_SET_VERSION)
}

fn operation_id(kind: &str, pack_id: &str) -> String {
    format!(
        "ysn-ocr-{kind}-{pack_id}-{}",
        chrono::Local::now().timestamp_millis()
    )
}

fn emit_pack_progress(
    app: &AppHandle,
    operation_id: &str,
    pack_id: &str,
    phase: &str,
    percent: u8,
    recoverable: bool,
    message: &str,
    next_action: Option<&str>,
) {
    let _ = app.emit(
        "ysn-ocr-model-pack-progress",
        json!({
            "operationId": operation_id,
            "packId": pack_id,
            "phase": phase,
            "percent": percent,
            "recoverable": recoverable,
            "message": message,
            "nextAction": next_action,
        }),
    );
}

async fn run_model_pack_operation(
    app: AppHandle,
    pack_id: String,
    kind: &str,
) -> Result<Value, String> {
    let operation_id = operation_id(kind, &pack_id);
    let root = model_root(&app)?;
    fs::create_dir_all(&root).map_err(|e| format!("failed to create OCR model directory: {e}"))?;

    emit_pack_progress(
        &app,
        &operation_id,
        &pack_id,
        "queued",
        5,
        true,
        "Model pack operation queued.",
        Some("resolve-model-index"),
    );
    let mut manifest = crate::ysn_ocr_manifest_store::read_manifest(&app)?;
    if let Some(source_error) =
        crate::ysn_ocr_model_sources::pack_source_blocker(&manifest, &pack_id)
    {
        let message = format!("Trusted model sources are not ready: {source_error}");
        let pack_dir = crate::ysn_ocr_manifest_store::write_pack_install_state(
            &app,
            &pack_id,
            &operation_id,
            "download-failed",
            &message,
        )?;
        crate::ysn_ocr_manifest_store::set_pack_status(
            &mut manifest,
            &pack_id,
            "download-failed",
            Some(&message),
        )?;
        crate::ysn_ocr_model_downloader::set_models_status(
            &mut manifest,
            &pack_id,
            "download-failed",
        );
        manifest["updatedAt"] = json!(crate::ysn_ocr_manifest_store::now_rfc3339());
        crate::ysn_ocr_manifest_store::write_manifest(&app, &manifest)?;
        emit_pack_progress(
            &app,
            &operation_id,
            &pack_id,
            "failed",
            100,
            true,
            &message,
            Some("configure-managed-model-sources"),
        );
        return Ok(
            json!({ "ok": false, "packId": pack_id, "modelDir": root.to_string_lossy().to_string(), "packDir": pack_dir.to_string_lossy().to_string(), "status": "download-failed", "operationId": operation_id, "phase": "failed", "recoverable": true, "nextAction": "configure-managed-model-sources", "message": message }),
        );
    }
    emit_pack_progress(
        &app,
        &operation_id,
        &pack_id,
        "resolving-index",
        20,
        true,
        "Resolving built-in model pack index.",
        Some("prepare-pack-directory"),
    );
    crate::ysn_ocr_manifest_store::set_pack_status(&mut manifest, &pack_id, "downloading", None)?;
    crate::ysn_ocr_model_downloader::set_models_status(&mut manifest, &pack_id, "downloading");
    crate::ysn_ocr_manifest_store::write_manifest(&app, &manifest)?;

    emit_pack_progress(
        &app,
        &operation_id,
        &pack_id,
        "downloading",
        35,
        true,
        "Resolving model download plan.",
        Some("verify-model-sources"),
    );
    let download_plan = match crate::ysn_ocr_manifest_store::collect_pack_download_plan(
        &manifest, &pack_id,
    ) {
        Ok(plan) if !plan.is_empty() => plan,
        Ok(_) => {
            let message = "Model pack has no downloadable model descriptors.";
            let pack_dir = crate::ysn_ocr_manifest_store::write_pack_install_state(
                &app,
                &pack_id,
                &operation_id,
                "download-failed",
                message,
            )?;
            crate::ysn_ocr_manifest_store::set_pack_status(
                &mut manifest,
                &pack_id,
                "download-failed",
                Some(message),
            )?;
            crate::ysn_ocr_model_downloader::set_models_status(
                &mut manifest,
                &pack_id,
                "download-failed",
            );
            manifest["updatedAt"] = json!(crate::ysn_ocr_manifest_store::now_rfc3339());
            crate::ysn_ocr_manifest_store::write_manifest(&app, &manifest)?;
            emit_pack_progress(
                &app,
                &operation_id,
                &pack_id,
                "failed",
                100,
                true,
                message,
                Some("configure-managed-download-source"),
            );
            return Ok(
                json!({ "ok": false, "packId": pack_id, "modelDir": root.to_string_lossy().to_string(), "packDir": pack_dir.to_string_lossy().to_string(), "status": "download-failed", "operationId": operation_id, "phase": "failed", "recoverable": true, "nextAction": "configure-managed-download-source", "message": message }),
            );
        }
        Err(error) => {
            let pack_dir = crate::ysn_ocr_manifest_store::write_pack_install_state(
                &app,
                &pack_id,
                &operation_id,
                "download-failed",
                &error,
            )?;
            crate::ysn_ocr_manifest_store::set_pack_status(
                &mut manifest,
                &pack_id,
                "download-failed",
                Some(&error),
            )?;
            crate::ysn_ocr_model_downloader::set_models_status(
                &mut manifest,
                &pack_id,
                "download-failed",
            );
            manifest["updatedAt"] = json!(crate::ysn_ocr_manifest_store::now_rfc3339());
            crate::ysn_ocr_manifest_store::write_manifest(&app, &manifest)?;
            emit_pack_progress(
                &app,
                &operation_id,
                &pack_id,
                "failed",
                100,
                true,
                &error,
                Some("configure-managed-download-source"),
            );
            return Ok(
                json!({ "ok": false, "packId": pack_id, "modelDir": root.to_string_lossy().to_string(), "packDir": pack_dir.to_string_lossy().to_string(), "status": "download-failed", "operationId": operation_id, "phase": "failed", "recoverable": true, "nextAction": "configure-managed-download-source", "message": error }),
            );
        }
    };

    let total = download_plan.len().max(1);
    for (index, plan) in download_plan.iter().enumerate() {
        let percent = 40 + (((index + 1) * 35) / total) as u8;
        let model_id = plan["modelId"].as_str().unwrap_or("unknown");
        let artifact_type = plan["artifactType"].as_str().unwrap_or("model");
        emit_pack_progress(
            &app,
            &operation_id,
            &pack_id,
            "downloading",
            percent,
            true,
            &format!("Downloading OCR {artifact_type} artifact for {model_id}."),
            Some("verify-sha256"),
        );
        if let Err(error) =
            crate::ysn_ocr_model_downloader::download_and_verify_artifact(&app, plan).await
        {
            let pack_dir = crate::ysn_ocr_manifest_store::write_pack_install_state(
                &app,
                &pack_id,
                &operation_id,
                "download-failed",
                &error,
            )?;
            crate::ysn_ocr_manifest_store::set_pack_status(
                &mut manifest,
                &pack_id,
                "download-failed",
                Some(&error),
            )?;
            crate::ysn_ocr_model_downloader::set_models_status(
                &mut manifest,
                &pack_id,
                "download-failed",
            );
            manifest["updatedAt"] = json!(crate::ysn_ocr_manifest_store::now_rfc3339());
            crate::ysn_ocr_manifest_store::write_manifest(&app, &manifest)?;
            emit_pack_progress(
                &app,
                &operation_id,
                &pack_id,
                "failed",
                100,
                true,
                &error,
                Some("retry-or-repair-model-pack"),
            );
            return Ok(
                json!({ "ok": false, "packId": pack_id, "modelDir": root.to_string_lossy().to_string(), "packDir": pack_dir.to_string_lossy().to_string(), "status": "download-failed", "operationId": operation_id, "phase": "failed", "recoverable": true, "nextAction": "retry-or-repair-model-pack", "message": error }),
            );
        }
    }

    emit_pack_progress(
        &app,
        &operation_id,
        &pack_id,
        "verifying",
        82,
        true,
        "All OCR artifact hashes verified.",
        Some("activate-model-pack"),
    );
    emit_pack_progress(
        &app,
        &operation_id,
        &pack_id,
        "installing",
        90,
        true,
        "Activating OCR model pack artifacts.",
        Some("self-test-model-pack"),
    );
    let pack_dir = crate::ysn_ocr_manifest_store::write_pack_install_state(
        &app,
        &pack_id,
        &operation_id,
        "installed",
        "Model pack artifacts installed and verified.",
    )?;
    crate::ysn_ocr_manifest_store::set_pack_status(&mut manifest, &pack_id, "installed", None)?;
    crate::ysn_ocr_model_downloader::set_models_status(&mut manifest, &pack_id, "installed");
    manifest["installedAt"] = json!(crate::ysn_ocr_manifest_store::now_rfc3339());
    manifest["updatedAt"] = json!(crate::ysn_ocr_manifest_store::now_rfc3339());
    crate::ysn_ocr_manifest_store::write_manifest(&app, &manifest)?;
    emit_pack_progress(
        &app,
        &operation_id,
        &pack_id,
        "completed",
        100,
        true,
        "Model pack artifacts installed and verified.",
        Some("run-self-test"),
    );

    Ok(json!({
        "ok": true,
        "packId": pack_id,
        "modelDir": root.to_string_lossy().to_string(),
        "packDir": pack_dir.to_string_lossy().to_string(),
        "status": "installed",
        "operationId": operation_id,
        "phase": "completed",
        "recoverable": true,
        "nextAction": "run-self-test",
        "message": "Model pack artifacts installed and verified."
    }))
}

#[tauri::command]
pub fn import_local_ysn_ocr_model(
    app: AppHandle,
    model_id: String,
    source_path: String,
) -> Result<Value, String> {
    let model_id = model_id.trim();
    if model_id.is_empty() {
        return Err("OCR model id is required.".to_string());
    }
    let source_path = PathBuf::from(source_path.trim());
    if !source_path.is_file() {
        return Err("Local OCR model file does not exist.".to_string());
    }

    let mut manifest = crate::ysn_ocr_manifest_store::read_manifest(&app)?;
    let model_index = manifest["models"]
        .as_array()
        .and_then(|models| {
            models
                .iter()
                .position(|model| model["id"].as_str() == Some(model_id))
        })
        .ok_or_else(|| format!("OCR model descriptor not found: {model_id}"))?;
    let relative_path = manifest["models"][model_index]["path"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let pack_id = manifest["models"][model_index]["packId"]
        .as_str()
        .unwrap_or("")
        .to_string();
    if !crate::ysn_ocr_model_downloader::is_safe_relative_model_path(&relative_path) {
        return Err(format!("OCR model path is unsafe: {model_id}"));
    }

    let active_path =
        crate::ysn_ocr_model_downloader::safe_active_model_path(&app, &relative_path)?;
    if let Some(parent) = active_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create active model directory: {e}"))?;
    }
    fs::copy(&source_path, &active_path)
        .map_err(|e| format!("failed to copy local OCR model into active directory: {e}"))?;
    let sha256 = crate::ysn_ocr_model_downloader::sha256_file(&active_path)?;
    let size = fs::metadata(&active_path)
        .map_err(|e| format!("failed to inspect imported OCR model: {e}"))?
        .len();
    let imported_at = crate::ysn_ocr_manifest_store::now_rfc3339();

    if let Some(models) = manifest["models"].as_array_mut() {
        let model = models
            .get_mut(model_index)
            .ok_or_else(|| format!("OCR model descriptor not found: {model_id}"))?;
        model["status"] = json!("installed");
        model["sha256"] = json!(sha256.clone());
        model["size"] = json!(size);
        model["source"] = json!({
            "provider": "local-dev",
            "url": source_path.to_string_lossy().to_string(),
            "license": "local-development"
        });
        model["importedAt"] = json!(imported_at.clone());
        model["lastTouchedAt"] = json!(imported_at.clone());
    }

    if !pack_id.is_empty() {
        let installed_model_ids: std::collections::HashSet<String> = manifest["models"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter(|model| model["status"].as_str() == Some("installed"))
            .filter_map(|model| model["id"].as_str().map(|id| id.to_string()))
            .collect();
        if let Some(packs) = manifest["packs"].as_array_mut() {
            if let Some(pack) = packs
                .iter_mut()
                .find(|pack| pack["id"].as_str() == Some(pack_id.as_str()))
            {
                let pack_model_ids: Vec<String> = pack["modelIds"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|value| value.as_str().map(|id| id.to_string()))
                    .collect();
                if !pack_model_ids.is_empty()
                    && pack_model_ids
                        .iter()
                        .all(|id| installed_model_ids.contains(id))
                {
                    pack["status"] = json!("installed");
                    pack["installedAt"] = json!(imported_at.clone());
                    pack["error"] = Value::Null;
                }
            }
        }
    }

    manifest["updatedAt"] = json!(imported_at.clone());
    crate::ysn_ocr_manifest_store::write_manifest(&app, &manifest)?;

    Ok(json!({
        "ok": true,
        "modelId": model_id,
        "packId": pack_id,
        "activePath": active_path.to_string_lossy().to_string(),
        "sha256": sha256,
        "size": size,
        "status": "installed",
        "sourceProvider": "local-dev",
        "message": "Local OCR model imported for development validation. Production downloads still require managed sources."
    }))
}

#[tauri::command]
pub fn dry_run_ysn_ocr_managed_source_index(
    app: AppHandle,
    index_path: String,
    pack_id: Option<String>,
) -> Result<Value, String> {
    let index_path = PathBuf::from(index_path.trim());
    if !index_path.is_file() {
        return Err("Managed OCR source index file does not exist.".to_string());
    }
    let content = fs::read_to_string(&index_path)
        .map_err(|e| format!("failed to read managed OCR source index: {e}"))?;
    let index: Value = serde_json::from_str(&content)
        .map_err(|e| format!("failed to parse managed OCR source index JSON: {e}"))?;
    let manifest = crate::ysn_ocr_manifest_store::read_manifest(&app)?;
    let pack_id = pack_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let result =
        crate::ysn_ocr_model_sources::dry_run_managed_source_index(&manifest, &index, pack_id)?;
    Ok(json!({
        "ok": result["ok"].clone(),
        "indexPath": index_path.to_string_lossy().to_string(),
        "packId": pack_id,
        "result": result,
        "message": "Managed OCR source index dry-run completed without writing manifest or activating models."
    }))
}
#[tauri::command]
pub fn import_ysn_ocr_managed_source_index(
    app: AppHandle,
    index_path: String,
) -> Result<Value, String> {
    let index_path = PathBuf::from(index_path.trim());
    if !index_path.is_file() {
        return Err("Managed OCR source index file does not exist.".to_string());
    }
    let content = fs::read_to_string(&index_path)
        .map_err(|e| format!("failed to read managed OCR source index: {e}"))?;
    let index: Value = serde_json::from_str(&content)
        .map_err(|e| format!("failed to parse managed OCR source index JSON: {e}"))?;
    let mut manifest = crate::ysn_ocr_manifest_store::read_manifest(&app)?;
    let result = crate::ysn_ocr_model_sources::apply_managed_source_index(&mut manifest, &index)?;
    crate::ysn_ocr_manifest_store::write_manifest(&app, &manifest)?;
    Ok(json!({
        "ok": true,
        "indexPath": index_path.to_string_lossy().to_string(),
        "manifestPath": crate::ysn_ocr_manifest_store::manifest_path(&app)?.to_string_lossy().to_string(),
        "updatedCount": result["updatedCount"].clone(),
        "updatedModels": result["updatedModels"].clone(),
        "sourceReadiness": result["sourceReadiness"].clone(),
        "message": "Managed OCR source index imported. Install or update model packs next."
    }))
}

#[tauri::command]
pub fn create_ysn_ocr_managed_source_index_template(app: AppHandle) -> Result<Value, String> {
    let manifest = crate::ysn_ocr_manifest_store::read_manifest(&app)?;
    let template = crate::ysn_ocr_model_sources::managed_source_index_template(&manifest);
    let template_dir = model_root(&app)?.join("source-index");
    fs::create_dir_all(&template_dir)
        .map_err(|e| format!("failed to create OCR source index template directory: {e}"))?;
    let template_path = template_dir.join("ysn-ocr-managed-source-index.template.json");
    let body = serde_json::to_string_pretty(&template)
        .map_err(|e| format!("failed to serialize managed OCR source index template: {e}"))?;
    fs::write(&template_path, body)
        .map_err(|e| format!("failed to write managed OCR source index template: {e}"))?;
    Ok(json!({
        "ok": true,
        "templatePath": template_path.to_string_lossy().to_string(),
        "templateDir": template_dir.to_string_lossy().to_string(),
        "modelCount": template["models"].as_array().map(|items| items.len()).unwrap_or(0),
        "message": "Managed OCR source index template created. Fill reviewed source metadata before importing."
    }))
}

#[tauri::command]
pub fn run_ysn_ocr_decode_fixture() -> Result<Value, String> {
    let probabilities = vec![
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.8, 0.9, 0.0, 0.0, 0.0, 0.7, 0.8, 0.0, 0.0, 0.0, 0.0, 0.0,
        0.6, 0.7, 0.0, 0.0, 0.0, 0.6, 0.8,
    ];
    let detector_config = crate::ysn_ocr_decode::DbTextDetectorConfig {
        probability_threshold: 0.5,
        minimum_area: 2,
        original_width: 50,
        original_height: 50,
    };
    let detections =
        crate::ysn_ocr_decode::decode_db_probability_map(&probabilities, 5, 5, &detector_config)?;
    let crop_plan = crate::ysn_ocr_crop::build_line_crop_plan(
        &detections,
        &crate::ysn_ocr_crop::OcrCropPlanConfig {
            image_width: 50,
            image_height: 50,
            padding: 2,
            minimum_width: 2,
            minimum_height: 2,
        },
    )?;
    let fixture_image: image::ImageBuffer<image::Rgb<u8>, Vec<u8>> =
        image::ImageBuffer::from_fn(50, 50, |x, y| {
            image::Rgb([(x % 255) as u8, (y % 255) as u8, ((x + y) % 255) as u8])
        });
    let mut fixture_image_bytes = Vec::new();
    fixture_image
        .write_to(
            &mut std::io::Cursor::new(&mut fixture_image_bytes),
            image::ImageFormat::Png,
        )
        .map_err(|error| format!("failed to encode OCR crop fixture image: {error}"))?;
    let cropped_lines =
        crate::ysn_ocr_crop::crop_line_images_from_bytes(&fixture_image_bytes, &crop_plan)?;
    let recognition_preprocess_config =
        crate::ysn_ocr_preprocess::OcrTensorPreprocessConfig::with_size_and_stats(
            32,
            8,
            [0.5, 0.5, 0.5],
            [0.5, 0.5, 0.5],
        )?;
    let recognition_inputs = cropped_lines
        .iter()
        .map(|crop| {
            crate::ysn_ocr_preprocess::cropped_line_to_nchw_rgb_tensor(
                crop,
                &recognition_preprocess_config,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let dictionary_content = "\nH\nello\nWorld\n";
    let dictionary_root = std::env::temp_dir().join(format!(
        "ysn-ocr-dictionary-fixture-{}",
        chrono::Local::now().timestamp_millis()
    ));
    let dictionary_dir = dictionary_root.join("dictionaries");
    fs::create_dir_all(&dictionary_dir)
        .map_err(|error| format!("failed to create OCR dictionary fixture directory: {error}"))?;
    let dictionary_path = dictionary_dir.join("latin.txt");
    fs::write(&dictionary_path, dictionary_content)
        .map_err(|error| format!("failed to write OCR dictionary fixture: {error}"))?;
    let dictionary_sha256 = crate::ysn_ocr_model_downloader::sha256_file(&dictionary_path)?;
    let dictionary_contract = json!({
        "dictionary": {
            "script": "latin",
            "path": "dictionaries/latin.txt",
            "sha256": dictionary_sha256,
            "size": dictionary_content.len() as u64,
            "blankTokenId": 0
        }
    });
    let loaded_dictionary = crate::ysn_ocr_dictionary::load_dictionary_from_contract(
        &dictionary_root,
        &dictionary_contract,
    )?;
    let dictionary = loaded_dictionary.tokens.clone();
    let _ = fs::remove_dir_all(&dictionary_root);
    let hello = crate::ysn_ocr_decode::decode_ctc_logits(
        &[0.1, 0.93, 0.2, 0.1, 0.8, 0.1, 0.1, 0.1, 0.1, 0.2, 0.91, 0.1],
        3,
        dictionary.len(),
        &dictionary,
        0,
    )?;
    let world = crate::ysn_ocr_decode::decode_ctc_logits(
        &[0.1, 0.1, 0.2, 0.89],
        1,
        dictionary.len(),
        &dictionary,
        0,
    )?;
    let blocks = crate::ysn_ocr_postprocess::align_detections_with_recognitions(
        &detections,
        &[hello, world],
        &crate::ysn_ocr_postprocess::OcrPostprocessConfig {
            minimum_confidence: 0.35,
            default_script: "latin".to_string(),
            default_language: "en".to_string(),
            model_id: "fixture-rec-latin".to_string(),
        },
    );
    let output_probe = crate::ysn_ocr_runtime_adapter::OnnxOutputProbe {
        name: "sigmoid_0.tmp_0".to_string(),
        element_type: Some("Float32".to_string()),
        shape: vec![1, 1, 5, 5],
        f32_tensor: Some(crate::ysn_ocr_runtime_adapter::summarize_f32_tensor(
            &[1, 1, 5, 5],
            &probabilities,
            8,
        )?),
    };
    let recognition_output_probe = crate::ysn_ocr_runtime_adapter::OnnxOutputProbe {
        name: "softmax_2.tmp_0".to_string(),
        element_type: Some("Float32".to_string()),
        shape: vec![1, 3, dictionary.len() as i64],
        f32_tensor: Some(crate::ysn_ocr_runtime_adapter::summarize_f32_tensor(
            &[1, 3, dictionary.len()],
            &[0.1, 0.9, 0.2, 0.1, 0.8, 0.1, 0.1, 0.2, 0.1, 0.2, 0.9, 0.1],
            8,
        )?),
    };
    let ctc_bridge_plan = crate::ysn_ocr_decode::build_ctc_logits_bridge_plan(
        &recognition_output_probe,
        &dictionary,
        0,
    );
    let model = json!({
        "id": "fixture-det-default",
        "type": "detection",
        "contract": { "decoder": { "type": "db-text-detector" } }
    });
    let pipeline_plan =
        crate::ysn_ocr_pipeline::build_decode_pipeline_plan(&model, &[output_probe]);
    Ok(json!({
        "ok": true,
        "fixture": "db-detector-plus-ctc-recognizer",
        "runtimeInferenceReady": false,
        "pipelinePlan": pipeline_plan,
        "cropPlan": crop_plan,
        "recognitionInputs": recognition_inputs,
        "ctcBridgePlan": ctc_bridge_plan,
        "dictionary": loaded_dictionary,
        "blocks": blocks,
        "message": "Decode/postprocess fixture executed. This is not a real ONNX OCR readiness signal."
    }))
}
#[tauri::command]
pub fn probe_ysn_ocr_model_session(app: AppHandle, relative_path: String) -> Result<Value, String> {
    let model_path =
        crate::ysn_ocr_model_downloader::safe_active_model_path(&app, relative_path.trim())?;
    let probe = crate::ysn_ocr_runtime_adapter::probe_onnx_session(&model_path)?;
    serde_json::to_value(probe).map_err(|e| format!("failed to serialize ONNX session probe: {e}"))
}

#[tauri::command]
pub fn probe_ysn_ocr_model_session_by_id(
    app: AppHandle,
    model_id: String,
) -> Result<Value, String> {
    let model_id = model_id.trim();
    if model_id.is_empty() {
        return Err("OCR model id is required.".to_string());
    }
    let manifest = crate::ysn_ocr_manifest_store::read_manifest(&app)?;
    let model = manifest["models"]
        .as_array()
        .and_then(|models| {
            models
                .iter()
                .find(|model| model["id"].as_str() == Some(model_id))
        })
        .ok_or_else(|| format!("OCR model descriptor not found: {model_id}"))?;
    let relative_path = model["path"].as_str().unwrap_or("");
    let model_path = crate::ysn_ocr_model_downloader::safe_active_model_path(&app, relative_path)?;
    let probe = crate::ysn_ocr_runtime_adapter::probe_onnx_session(&model_path)?;
    serde_json::to_value(probe).map_err(|e| format!("failed to serialize ONNX session probe: {e}"))
}

#[tauri::command]
pub fn probe_ysn_ocr_model_session_readiness_by_id(
    app: AppHandle,
    model_id: String,
) -> Result<Value, String> {
    let model_id = model_id.trim();
    if model_id.is_empty() {
        return Err("OCR model id is required.".to_string());
    }
    let manifest = crate::ysn_ocr_manifest_store::read_manifest(&app)?;
    let model = manifest["models"]
        .as_array()
        .and_then(|models| {
            models
                .iter()
                .find(|model| model["id"].as_str() == Some(model_id))
        })
        .ok_or_else(|| format!("OCR model descriptor not found: {model_id}"))?;
    let relative_path = model["path"].as_str().unwrap_or("");
    let model_path = crate::ysn_ocr_model_downloader::safe_active_model_path(&app, relative_path)?;
    Ok(crate::ysn_ocr_runtime_adapter::probe_onnx_session_readiness(&model_path))
}

#[tauri::command]
pub fn run_ysn_ocr_model_inference_probe(
    app: AppHandle,
    model_id: String,
    image_path: String,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<Value, String> {
    let model_id = model_id.trim();
    if model_id.is_empty() {
        return Err("OCR model id is required.".to_string());
    }
    let image_path = PathBuf::from(image_path.trim());
    if !image_path.is_file() {
        return Err("OCR inference probe image file does not exist.".to_string());
    }
    let manifest = crate::ysn_ocr_manifest_store::read_manifest(&app)?;
    let model = manifest["models"]
        .as_array()
        .and_then(|models| {
            models
                .iter()
                .find(|model| model["id"].as_str() == Some(model_id))
        })
        .ok_or_else(|| format!("OCR model descriptor not found: {model_id}"))?;
    let relative_path = model["path"].as_str().unwrap_or("");
    let model_path = crate::ysn_ocr_model_downloader::safe_active_model_path(&app, relative_path)?;
    let image_bytes = fs::read(&image_path)
        .map_err(|error| format!("failed to read OCR inference probe image: {error}"))?;
    let config = crate::ysn_ocr_preprocess::OcrTensorPreprocessConfig::for_model_descriptor(
        model, width, height,
    )?;
    let tensor = crate::ysn_ocr_preprocess::image_bytes_to_nchw_rgb_tensor(&image_bytes, &config)?;
    let probe = crate::ysn_ocr_runtime_adapter::run_onnx_nchw_f32_probe(&model_path, &tensor)?;
    Ok(json!({
        "ok": probe.ok,
        "modelId": model_id,
        "imagePath": image_path.to_string_lossy().to_string(),
        "preprocess": tensor,
        "probe": probe,
        "status": "inference-scaffold-executed; OCR detection/recognition postprocessing is not implemented yet"
    }))
}

#[tauri::command]
pub fn plan_ysn_ocr_routes(app: AppHandle, texts: Value) -> Result<Value, String> {
    let route_texts: Vec<String> = texts
        .as_array()
        .ok_or_else(|| "OCR route plan expects texts to be a JSON array.".to_string())?
        .iter()
        .map(|value| value.as_str().unwrap_or_default().to_string())
        .collect();
    let manifest = crate::ysn_ocr_manifest_store::read_manifest(&app)?;
    Ok(crate::ysn_ocr_router::build_route_plan(
        &manifest,
        &route_texts,
    ))
}

#[tauri::command]
pub fn get_ysn_ocr_model_index() -> Result<Value, String> {
    Ok({
        let mut index = default_model_index();
        index["sourcePolicy"] = crate::ysn_ocr_model_sources::trusted_source_policy();
        index["schemaIssues"] = json!(crate::ysn_ocr_model_schema::validate_model_index_schema(
            &index
        ));
        index
    })
}

fn build_readiness_steps(
    source_readiness: &Value,
    manifest_issues: &[Value],
    required_count: usize,
    installed_required: usize,
    broken: &[String],
    active_model_issues: &[Value],
    runtime_inference_ready: bool,
    manifest: &Value,
) -> Value {
    let source_ready = source_readiness["ready"].as_bool().unwrap_or(false);
    let manifest_ready = manifest_issues
        .iter()
        .all(|issue| issue["severity"].as_str() != Some("error"));
    let model_packs_installed =
        required_count > 0 && installed_required == required_count && broken.is_empty();
    let active_models_ready = model_packs_installed && active_model_issues.is_empty();
    let self_test_ready = manifest["lastSelfTestAt"].as_str().is_some()
        && active_models_ready
        && runtime_inference_ready;

    json!([
        {
            "id": "trusted-sources",
            "ready": source_ready,
            "severity": if source_ready { "success" } else { "warning" },
            "label": "Trusted model sources",
            "description": if source_ready { "Production managed model sources are configured." } else { "Managed model source metadata is incomplete." },
            "nextAction": if source_ready { "install-or-update-model-packs" } else { source_readiness["nextAction"].as_str().unwrap_or("configure-managed-model-sources") }
        },
        {
            "id": "manifest",
            "ready": manifest_ready,
            "severity": if manifest_ready { "success" } else { "error" },
            "label": "Manifest integrity",
            "description": if manifest_ready { "Model manifest has no blocking schema errors." } else { "Model manifest has blocking errors." },
            "nextAction": if manifest_ready { "continue" } else { "repair-model-manifest" }
        },
        {
            "id": "model-packs",
            "ready": model_packs_installed,
            "severity": if model_packs_installed { "success" } else { "warning" },
            "label": "Required model packs",
            "description": format!("{installed_required}/{required_count} required packs installed."),
            "nextAction": if model_packs_installed { "run-active-model-health-check" } else { "install-or-repair-model-packs" }
        },
        {
            "id": "active-models",
            "ready": active_models_ready,
            "severity": if active_models_ready { "success" } else { "warning" },
            "label": "Active model files",
            "description": if active_models_ready { "Active model files are present and verified.".to_string() } else if !model_packs_installed { "Required model packs are not installed yet.".to_string() } else { format!("{} active model issues need recovery.", active_model_issues.len()) },
            "nextAction": if active_models_ready { "run-ocr-self-test" } else if !model_packs_installed { "install-or-repair-model-packs" } else { "repair-active-model-files" }
        },
        {
            "id": "runtime-inference",
            "ready": runtime_inference_ready,
            "severity": if runtime_inference_ready { "success" } else { "warning" },
            "label": "ONNX inference runtime",
            "description": if runtime_inference_ready { "ONNX inference runtime is enabled." } else { "ONNX inference runtime is still scaffolded and not production-ready." },
            "nextAction": if runtime_inference_ready { "run-ocr-self-test" } else { "complete-onnx-inference-runtime" }
        },
        {
            "id": "self-test",
            "ready": self_test_ready,
            "severity": if self_test_ready { "success" } else { "warning" },
            "label": "OCR self-test",
            "description": if self_test_ready { "Latest OCR Runtime self-test passed." } else { "OCR Runtime self-test has not passed for the current runtime state." },
            "nextAction": if self_test_ready { "ready" } else { "run-ocr-self-test" }
        }
    ])
}

#[tauri::command]
pub fn get_ysn_ocr_status(app: AppHandle) -> Result<Value, String> {
    let root = model_root(&app)?;
    let manifest = crate::ysn_ocr_manifest_store::read_manifest(&app)?;
    let packs = manifest["packs"].as_array().cloned().unwrap_or_default();
    let manifest_issues = crate::ysn_ocr_manifest_store::validate_manifest(&manifest);
    let source_readiness = crate::ysn_ocr_model_sources::source_readiness(&manifest);
    let required_count = packs
        .iter()
        .filter(|pack| pack["required"].as_bool().unwrap_or(false))
        .count();
    let installed_required = packs
        .iter()
        .filter(|pack| {
            pack["required"].as_bool().unwrap_or(false)
                && pack["status"].as_str() == Some("installed")
        })
        .count();
    let broken: Vec<String> = packs
        .iter()
        .filter(|pack| {
            matches!(
                pack["status"].as_str(),
                Some(
                    "download-failed"
                        | "verify-failed"
                        | "install-failed"
                        | "self-test-failed"
                        | "broken"
                )
            )
        })
        .filter_map(|pack| pack["id"].as_str().map(|id| id.to_string()))
        .collect();
    let active_model_health = crate::ysn_ocr_model_downloader::active_model_health(&app, &manifest);
    let active_model_issues: Vec<Value> = active_model_health
        .iter()
        .filter(|item| !item["ok"].as_bool().unwrap_or(false))
        .cloned()
        .collect();

    let model_packs_ready = required_count > 0
        && installed_required == required_count
        && broken.is_empty()
        && active_model_issues.is_empty();
    let runtime_inference_ready = false;
    let readiness_steps = build_readiness_steps(
        &source_readiness,
        &manifest_issues,
        required_count,
        installed_required,
        &broken,
        &active_model_issues,
        runtime_inference_ready,
        &manifest,
    );
    let source_ready = source_readiness["ready"].as_bool().unwrap_or(false);
    let manifest_ready = manifest_issues
        .iter()
        .all(|issue| issue["severity"].as_str() != Some("error"));
    let active_models_ready = required_count > 0
        && installed_required == required_count
        && broken.is_empty()
        && active_model_issues.is_empty();
    let self_test_ready = manifest["lastSelfTestAt"].as_str().is_some()
        && active_models_ready
        && runtime_inference_ready;

    Ok(json!({
        "ready": runtime_inference_ready && model_packs_ready,
        "sourceReady": source_ready,
        "manifestReady": manifest_ready,
        "modelPacksReady": model_packs_ready,
        "activeModelsReady": active_models_ready,
        "runtimeInferenceReady": runtime_inference_ready,
        "selfTestReady": self_test_ready,
        "readinessSteps": readiness_steps,
        "runtime": "ysn-ocr-runtime",
        "runtimeVersion": RUNTIME_VERSION,
        "modelSetVersion": manifest["modelSetVersion"].clone(),
        "modelDir": root.to_string_lossy().to_string(),
        "defaultSourceLanguage": "auto",
        "defaultProfile": manifest["defaultProfile"].clone(),
        "installedRequiredPacks": installed_required,
        "requiredPacks": required_count,
        "brokenPacks": broken,
        "activeModelHealth": active_model_health,
        "activeModelIssues": active_model_issues,
        "manifestIssues": manifest_issues,
        "sourceReadiness": source_readiness,
        "manifest": manifest,
        "implementationStatus": "managed-manifest-skeleton"
    }))
}

#[tauri::command]
pub fn run_ysn_ocr_self_test(app: AppHandle) -> Result<Value, String> {
    let tested_at = crate::ysn_ocr_manifest_store::now_rfc3339();
    let mut manifest = crate::ysn_ocr_manifest_store::read_manifest(&app)?;
    let manifest_issues = crate::ysn_ocr_manifest_store::validate_manifest(&manifest);
    let blocking_issues: Vec<Value> = manifest_issues
        .iter()
        .filter(|issue| issue["severity"].as_str() == Some("error"))
        .cloned()
        .collect();
    let missing_active_models =
        crate::ysn_ocr_model_downloader::active_model_missing(&app, &manifest);
    manifest["lastSelfTestAt"] = json!(tested_at.clone());
    crate::ysn_ocr_manifest_store::set_installed_pack_self_test_time(&mut manifest, &tested_at);
    crate::ysn_ocr_manifest_store::write_manifest(&app, &manifest)?;

    let status = get_ysn_ocr_status(app)?;
    let model_packs_ready = status["modelPacksReady"].as_bool().unwrap_or(false)
        && blocking_issues.is_empty()
        && missing_active_models.is_empty();
    let runtime_inference_ready = status["runtimeInferenceReady"].as_bool().unwrap_or(false);
    let ok = model_packs_ready && runtime_inference_ready;
    let message = if ok {
        "YSN OCR Runtime self-test passed."
    } else if !model_packs_ready {
        "Model pack health check did not pass. Install or repair required OCR model packs."
    } else {
        "Model pack health check passed, but ONNX inference runtime is not enabled yet."
    };

    Ok(json!({
        "ok": ok,
        "modelPacksReady": model_packs_ready,
        "runtimeInferenceReady": runtime_inference_ready,
        "testedAt": tested_at,
        "runtime": "ysn-ocr-runtime",
        "message": message,
        "manifestIssues": manifest_issues,
        "missingActiveModels": missing_active_models,
        "samples": []
    }))
}

#[tauri::command]
pub async fn install_ysn_ocr_model_pack(app: AppHandle, pack_id: String) -> Result<Value, String> {
    run_model_pack_operation(app, pack_id, "install").await
}

#[tauri::command]
pub async fn update_ysn_ocr_model_pack(app: AppHandle, pack_id: String) -> Result<Value, String> {
    run_model_pack_operation(app, pack_id, "update").await
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn test_readiness_steps_do_not_mark_missing_sources_ready() {
        let source_readiness =
            json!({ "ready": false, "nextAction": "configure-managed-model-sources" });
        let manifest = json!({ "lastSelfTestAt": null });
        let steps =
            super::build_readiness_steps(&source_readiness, &[], 1, 0, &[], &[], false, &manifest);
        let trusted = steps
            .as_array()
            .unwrap()
            .iter()
            .find(|step| step["id"] == "trusted-sources")
            .unwrap();
        let runtime = steps
            .as_array()
            .unwrap()
            .iter()
            .find(|step| step["id"] == "runtime-inference")
            .unwrap();
        assert_eq!(trusted["ready"].as_bool(), Some(false));
        assert_eq!(
            trusted["nextAction"].as_str(),
            Some("configure-managed-model-sources")
        );
        assert_eq!(runtime["ready"].as_bool(), Some(false));
    }

    #[test]
    fn test_readiness_steps_keep_runtime_not_ready_after_pack_health() {
        let source_readiness =
            json!({ "ready": true, "nextAction": "install-or-self-test-model-packs" });
        let manifest = json!({ "lastSelfTestAt": "2026-06-02T00:00:00+08:00" });
        let steps =
            super::build_readiness_steps(&source_readiness, &[], 1, 1, &[], &[], false, &manifest);
        let packs = steps
            .as_array()
            .unwrap()
            .iter()
            .find(|step| step["id"] == "model-packs")
            .unwrap();
        let active = steps
            .as_array()
            .unwrap()
            .iter()
            .find(|step| step["id"] == "active-models")
            .unwrap();
        let runtime = steps
            .as_array()
            .unwrap()
            .iter()
            .find(|step| step["id"] == "runtime-inference")
            .unwrap();
        let self_test = steps
            .as_array()
            .unwrap()
            .iter()
            .find(|step| step["id"] == "self-test")
            .unwrap();
        assert_eq!(packs["ready"].as_bool(), Some(true));
        assert_eq!(active["ready"].as_bool(), Some(true));
        assert_eq!(runtime["ready"].as_bool(), Some(false));
        assert_eq!(self_test["ready"].as_bool(), Some(false));
    }
}

use futures_util::StreamExt;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tauri::AppHandle;

use crate::ysn_ocr_runtime::model_root;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OcrArtifactDownloadPlan {
    pub artifact_type: String,
    pub model_id: String,
    pub url: String,
    pub expected_sha256: String,
    pub expected_size: u64,
    pub relative_path: String,
}

#[derive(Debug, Clone, PartialEq)]
struct OcrActiveArtifactSpec {
    artifact_type: String,
    model_id: String,
    pack_id: Value,
    relative_path: String,
    expected_sha256: String,
    expected_size: u64,
    source_provider: String,
}

pub(crate) fn downloads_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(model_root(app)?.join("downloads"))
}

pub(crate) fn active_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(model_root(app)?.join("active"))
}

pub(crate) fn safe_active_model_path(
    app: &AppHandle,
    relative_path: &str,
) -> Result<PathBuf, String> {
    safe_active_artifact_path(app, "model", relative_path)
}

pub(crate) fn safe_active_artifact_path(
    app: &AppHandle,
    artifact_type: &str,
    relative_path: &str,
) -> Result<PathBuf, String> {
    safe_active_artifact_path_from_root(&active_dir(app)?, artifact_type, relative_path)
}

pub(crate) fn safe_active_artifact_path_from_root(
    active_root: &Path,
    artifact_type: &str,
    relative_path: &str,
) -> Result<PathBuf, String> {
    if !is_safe_artifact_path(artifact_type, relative_path) {
        return Err(format!(
            "unsafe OCR {artifact_type} artifact path: {relative_path}"
        ));
    }
    Ok(active_root.join(relative_path))
}

pub(crate) fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file =
        fs::File::open(path).map_err(|e| format!("failed to open file for SHA256: {e}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 64];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|e| format!("failed to read file for SHA256: {e}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub(crate) fn is_safe_relative_model_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    !normalized.trim().is_empty()
        && !normalized.starts_with('/')
        && !normalized.contains(":")
        && !normalized.split('/').any(|part| part == "..")
}

fn is_safe_artifact_path(artifact_type: &str, path: &str) -> bool {
    match artifact_type {
        "model" => is_safe_relative_model_path(path),
        "dictionary" => crate::ysn_ocr_dictionary::is_safe_dictionary_path(path),
        _ => false,
    }
}

pub(crate) fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|ch| ch.is_ascii_hexdigit())
}

async fn download_url_to_file(url: &str, path: &Path, artifact_type: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create download directory: {e}"))?;
    }
    let response = reqwest::get(url)
        .await
        .map_err(|e| format!("failed to start OCR {artifact_type} download: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "OCR {artifact_type} download failed with HTTP status {}",
            response.status()
        ));
    }
    let mut file = fs::File::create(path)
        .map_err(|e| format!("failed to create OCR {artifact_type} download file: {e}"))?;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let bytes = chunk
            .map_err(|e| format!("failed while downloading OCR {artifact_type} bytes: {e}"))?;
        file.write_all(&bytes)
            .map_err(|e| format!("failed to write OCR {artifact_type} download bytes: {e}"))?;
    }
    file.flush()
        .map_err(|e| format!("failed to flush OCR {artifact_type} download file: {e}"))?;
    Ok(())
}

pub(crate) async fn download_and_verify_artifact(
    app: &AppHandle,
    plan: &Value,
) -> Result<PathBuf, String> {
    let artifact_plan = parse_artifact_download_plan(plan)?;
    let download_path = downloads_dir(app)?.join(format!(
        "{}-{}-{}.download",
        artifact_plan.artifact_type,
        artifact_plan.model_id,
        chrono::Local::now().timestamp_millis()
    ));
    download_url_to_file(
        &artifact_plan.url,
        &download_path,
        &artifact_plan.artifact_type,
    )
    .await?;
    verify_and_activate_downloaded_artifact(&download_path, &active_dir(app)?, &artifact_plan)
}

pub(crate) fn verify_and_activate_downloaded_artifact(
    download_path: &Path,
    active_root: &Path,
    artifact_plan: &OcrArtifactDownloadPlan,
) -> Result<PathBuf, String> {
    let actual_size = fs::metadata(download_path)
        .map_err(|e| format!("failed to inspect downloaded OCR artifact size: {e}"))?
        .len();
    if actual_size != artifact_plan.expected_size {
        let _ = fs::remove_file(download_path);
        return Err(format!(
            "size mismatch for OCR {} artifact {}",
            artifact_plan.artifact_type, artifact_plan.model_id
        ));
    }
    let actual_sha256 = sha256_file(download_path)?;
    if !actual_sha256.eq_ignore_ascii_case(&artifact_plan.expected_sha256) {
        let _ = fs::remove_file(download_path);
        return Err(format!(
            "SHA256 mismatch for OCR {} artifact {}",
            artifact_plan.artifact_type, artifact_plan.model_id
        ));
    }
    let active_path = safe_active_artifact_path_from_root(
        active_root,
        &artifact_plan.artifact_type,
        &artifact_plan.relative_path,
    )?;
    if let Some(parent) = active_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create active OCR artifact directory: {e}"))?;
    }
    if active_path.exists() {
        let _ = fs::remove_file(&active_path);
    }
    fs::rename(download_path, &active_path)
        .map_err(|e| format!("failed to activate downloaded OCR artifact: {e}"))?;
    Ok(active_path)
}

pub(crate) fn parse_artifact_download_plan(
    plan: &Value,
) -> Result<OcrArtifactDownloadPlan, String> {
    let model_id = plan["modelId"]
        .as_str()
        .ok_or_else(|| "download plan missing modelId".to_string())?;
    let artifact_type = plan["artifactType"].as_str().unwrap_or("model");
    let url = plan["url"]
        .as_str()
        .ok_or_else(|| format!("download plan missing url: {model_id}"))?;
    let expected_sha256 = plan["sha256"]
        .as_str()
        .ok_or_else(|| format!("download plan missing sha256: {model_id}"))?;
    let expected_size = plan["size"]
        .as_u64()
        .ok_or_else(|| format!("download plan missing size: {model_id}"))?;
    let relative_path = plan["relativePath"]
        .as_str()
        .ok_or_else(|| format!("download plan missing relative path: {model_id}"))?;
    if expected_size == 0 {
        return Err(format!(
            "download plan has invalid OCR {artifact_type} size: {model_id}"
        ));
    }
    if !is_safe_artifact_path(artifact_type, relative_path) {
        return Err(format!(
            "download plan has unsafe OCR {artifact_type} artifact path: {relative_path}"
        ));
    }
    if !is_sha256_hex(expected_sha256) {
        return Err(format!(
            "download plan has invalid OCR {artifact_type} SHA256: {model_id}"
        ));
    }
    Ok(OcrArtifactDownloadPlan {
        artifact_type: artifact_type.to_string(),
        model_id: model_id.to_string(),
        url: url.to_string(),
        expected_sha256: expected_sha256.to_string(),
        expected_size,
        relative_path: relative_path.to_string(),
    })
}

pub(crate) fn set_models_status(manifest: &mut Value, pack_id: &str, status: &str) {
    if let Some(models) = manifest["models"].as_array_mut() {
        for model in models
            .iter_mut()
            .filter(|model| model["packId"].as_str() == Some(pack_id))
        {
            model["status"] = json!(status);
            model["lastTouchedAt"] = json!(chrono::Local::now().to_rfc3339());
        }
    }
}

pub(crate) fn active_model_missing(app: &AppHandle, manifest: &Value) -> Vec<String> {
    let health = active_model_health(app, manifest);
    active_artifact_blocker_ids(&health)
}

pub(crate) fn active_artifact_blocker_ids(health: &[Value]) -> Vec<String> {
    health
        .iter()
        .filter(|item| !item["ok"].as_bool().unwrap_or(false))
        .map(|item| {
            let model_id = item["modelId"].as_str().unwrap_or("unknown");
            let artifact_type = item["artifactType"].as_str().unwrap_or("model");
            format!("{model_id}:{artifact_type}")
        })
        .collect()
}

pub(crate) fn active_model_health(app: &AppHandle, manifest: &Value) -> Vec<Value> {
    manifest["models"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter(|model| model["status"].as_str() == Some("installed"))
        .flat_map(|model| active_artifact_health_for_model(app, model))
        .collect()
}

fn active_artifact_health_for_model(app: &AppHandle, model: &Value) -> Vec<Value> {
    active_artifact_specs_for_model(model)
        .iter()
        .map(|spec| {
            active_single_artifact_health(
                app,
                &spec.model_id,
                spec.pack_id.clone(),
                &spec.artifact_type,
                &spec.relative_path,
                &spec.expected_sha256,
                spec.expected_size,
                &spec.source_provider,
            )
        })
        .collect()
}

fn active_artifact_specs_for_model(model: &Value) -> Vec<OcrActiveArtifactSpec> {
    let model_id = model["id"].as_str().unwrap_or("unknown").to_string();
    let mut specs = vec![OcrActiveArtifactSpec {
        artifact_type: "model".to_string(),
        model_id: model_id.clone(),
        pack_id: model["packId"].clone(),
        relative_path: model["path"].as_str().unwrap_or("").to_string(),
        expected_sha256: model["sha256"].as_str().unwrap_or("").to_string(),
        expected_size: model["size"].as_u64().unwrap_or(0),
        source_provider: model["source"]["provider"]
            .as_str()
            .unwrap_or("unknown")
            .to_string(),
    }];
    if model["type"].as_str() == Some("recognition") {
        let dictionary = &model["contract"]["dictionary"];
        specs.push(OcrActiveArtifactSpec {
            artifact_type: "dictionary".to_string(),
            model_id,
            pack_id: model["packId"].clone(),
            relative_path: dictionary["path"].as_str().unwrap_or("").to_string(),
            expected_sha256: dictionary["sha256"].as_str().unwrap_or("").to_string(),
            expected_size: dictionary["size"].as_u64().unwrap_or(0),
            source_provider: dictionary["source"]["provider"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
        });
    }
    specs
}
fn active_single_artifact_health(
    app: &AppHandle,
    model_id: &str,
    pack_id: Value,
    artifact_type: &str,
    relative_path: &str,
    expected_sha256: &str,
    expected_size: u64,
    source_provider: &str,
) -> Value {
    let mut issues = Vec::new();
    let mut exists = false;
    let mut actual_sha256 = Value::Null;
    let mut actual_size = Value::Null;
    let mut active_path_value = Value::Null;

    match safe_active_artifact_path(app, artifact_type, relative_path) {
        Ok(active_path) => {
            active_path_value = json!(active_path.to_string_lossy().to_string());
            exists = active_path.is_file();
            if exists {
                match fs::metadata(&active_path) {
                    Ok(metadata) => {
                        let size = metadata.len();
                        if expected_size > 0 && size != expected_size {
                            issues.push(json!({ "code": "size-mismatch", "message": format!("Active OCR {artifact_type} size does not match manifest.") }));
                        }
                        actual_size = json!(size);
                    }
                    Err(error) => issues
                        .push(json!({ "code": "size-read-failed", "message": error.to_string() })),
                }
                match sha256_file(&active_path) {
                    Ok(hash) => {
                        if is_sha256_hex(expected_sha256)
                            && !hash.eq_ignore_ascii_case(expected_sha256)
                        {
                            issues.push(json!({ "code": "sha256-mismatch", "message": format!("Active OCR {artifact_type} SHA256 does not match manifest.") }));
                        }
                        actual_sha256 = json!(hash);
                    }
                    Err(error) => {
                        issues.push(json!({ "code": "sha256-read-failed", "message": error }))
                    }
                }
            } else {
                issues.push(json!({ "code": "active-file-missing", "message": format!("Active OCR {artifact_type} file is missing.") }));
            }
        }
        Err(error) => issues.push(json!({ "code": "unsafe-active-path", "message": error })),
    }

    if source_provider != "ysn-managed" {
        issues.push(json!({ "code": "non-production-source", "message": format!("OCR {artifact_type} was not installed from a production managed source.") }));
    }

    json!({
        "artifactType": artifact_type,
        "modelId": model_id,
        "packId": pack_id,
        "relativePath": relative_path,
        "activePath": active_path_value,
        "exists": exists,
        "expectedSha256": expected_sha256,
        "actualSha256": actual_sha256,
        "expectedSize": expected_size,
        "actualSize": actual_size,
        "sourceProvider": source_provider,
        "productionSource": source_provider == "ysn-managed",
        "ok": issues.is_empty(),
        "issues": issues
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use std::fs;

    #[test]
    fn parse_artifact_download_plan_defaults_legacy_model_artifact() {
        let plan = super::parse_artifact_download_plan(&json!({
            "modelId": "rec-latin",
            "url": "https://models.example.invalid/rec-latin.onnx",
            "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "size": 128,
            "relativePath": "models/rec-latin.onnx"
        }))
        .unwrap();

        assert_eq!(plan.artifact_type, "model");
        assert_eq!(plan.relative_path, "models/rec-latin.onnx");
    }

    #[test]
    fn parse_artifact_download_plan_accepts_dictionary_artifact() {
        let plan = super::parse_artifact_download_plan(&json!({
            "artifactType": "dictionary",
            "modelId": "rec-latin",
            "url": "https://models.example.invalid/dictionaries/latin.txt",
            "sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "size": 64,
            "relativePath": "dictionaries/latin.txt"
        }))
        .unwrap();

        assert_eq!(plan.artifact_type, "dictionary");
        assert_eq!(plan.relative_path, "dictionaries/latin.txt");
    }

    #[test]
    fn parse_artifact_download_plan_rejects_dictionary_escape_path() {
        let error = super::parse_artifact_download_plan(&json!({
            "artifactType": "dictionary",
            "modelId": "rec-latin",
            "url": "https://models.example.invalid/latin.txt",
            "sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "size": 64,
            "relativePath": "../latin.txt"
        }))
        .unwrap_err();

        assert!(error.contains("unsafe"));
    }

    #[test]
    fn parse_artifact_download_plan_rejects_unknown_artifact_type() {
        let error = super::parse_artifact_download_plan(&json!({
            "artifactType": "weights",
            "modelId": "rec-latin",
            "url": "https://models.example.invalid/weights.bin",
            "sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "size": 128,
            "relativePath": "models/weights.bin"
        }))
        .unwrap_err();

        assert!(error.contains("unsafe"));
    }

    #[test]
    fn parse_artifact_download_plan_rejects_invalid_sha256() {
        let error = super::parse_artifact_download_plan(&json!({
            "artifactType": "model",
            "modelId": "rec-latin",
            "url": "https://models.example.invalid/rec-latin.onnx",
            "sha256": "bad",
            "size": 128,
            "relativePath": "models/rec-latin.onnx"
        }))
        .unwrap_err();

        assert!(error.contains("invalid"));
    }

    #[test]
    fn parse_artifact_download_plan_rejects_missing_size() {
        let error = super::parse_artifact_download_plan(&json!({
            "artifactType": "model",
            "modelId": "rec-latin",
            "url": "https://models.example.invalid/rec-latin.onnx",
            "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "relativePath": "models/rec-latin.onnx"
        }))
        .unwrap_err();

        assert!(error.contains("size"));
    }

    #[test]
    fn parse_artifact_download_plan_rejects_zero_size() {
        let error = super::parse_artifact_download_plan(&json!({
            "artifactType": "dictionary",
            "modelId": "rec-latin",
            "url": "https://models.example.invalid/dictionaries/latin.txt",
            "sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "size": 0,
            "relativePath": "dictionaries/latin.txt"
        }))
        .unwrap_err();

        assert!(error.contains("size"));
    }

    #[test]
    fn verify_and_activate_downloaded_artifact_rejects_size_mismatch() {
        let root = std::env::temp_dir().join(format!(
            "ysn-ocr-size-mismatch-{}",
            chrono::Local::now()
                .timestamp_nanos_opt()
                .unwrap_or_default()
        ));
        let download_path = root.join("downloads/model.download");
        let active_root = root.join("active");
        fs::create_dir_all(download_path.parent().unwrap()).unwrap();
        fs::write(&download_path, b"actual-bytes").unwrap();
        let plan = super::OcrArtifactDownloadPlan {
            artifact_type: "model".to_string(),
            model_id: "rec-latin".to_string(),
            url: "https://models.example.invalid/rec-latin.onnx".to_string(),
            expected_sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_string(),
            expected_size: 999,
            relative_path: "models/rec-latin.onnx".to_string(),
        };

        let error =
            super::verify_and_activate_downloaded_artifact(&download_path, &active_root, &plan)
                .unwrap_err();

        assert!(error.contains("size mismatch"));
        assert!(!download_path.exists());
        assert!(!active_root.join("models/rec-latin.onnx").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn active_artifact_blocker_ids_include_size_and_sha_issues() {
        let health = vec![
            json!({
                "artifactType": "model",
                "modelId": "rec-latin",
                "ok": true,
                "issues": []
            }),
            json!({
                "artifactType": "dictionary",
                "modelId": "rec-latin",
                "ok": false,
                "exists": true,
                "issues": [{ "code": "size-mismatch" }]
            }),
            json!({
                "artifactType": "model",
                "modelId": "rec-cjk",
                "ok": false,
                "exists": true,
                "issues": [{ "code": "sha256-mismatch" }]
            }),
        ];

        let blockers = super::active_artifact_blocker_ids(&health);

        assert_eq!(blockers, vec!["rec-latin:dictionary", "rec-cjk:model"]);
    }

    #[test]
    fn active_artifact_specs_include_dictionary_for_recognition_model() {
        let mut model = json!({
            "id": "rec-latin",
            "type": "recognition",
            "packId": "core-latin",
            "path": "models/rec-latin.onnx",
            "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "source": { "provider": "ysn-managed" },
            "contract": crate::ysn_ocr_model_schema::recognition_model_contract("latin")
        });
        model["contract"]["dictionary"]["sha256"] =
            json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");

        let specs = super::active_artifact_specs_for_model(&model);

        assert_eq!(specs.len(), 2);
        assert_eq!(specs[0].artifact_type, "model");
        assert_eq!(specs[1].artifact_type, "dictionary");
        assert_eq!(specs[1].relative_path, "dictionaries/latin.txt");
    }

    #[test]
    fn active_artifact_specs_do_not_add_dictionary_for_detector() {
        let model = json!({
            "id": "det-default",
            "type": "detection",
            "packId": "core-latin",
            "path": "models/det-default.onnx",
            "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "source": { "provider": "ysn-managed" },
            "contract": crate::ysn_ocr_model_schema::detection_model_contract()
        });

        let specs = super::active_artifact_specs_for_model(&model);

        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].artifact_type, "model");
    }
}

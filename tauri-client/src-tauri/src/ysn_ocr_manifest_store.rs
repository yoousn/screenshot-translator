use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use tauri::AppHandle;

use crate::ysn_ocr_runtime::{model_root, MODEL_SET_VERSION, RUNTIME_VERSION};

fn default_manifest() -> Value {
    crate::ysn_ocr_model_index::default_manifest(RUNTIME_VERSION, MODEL_SET_VERSION)
}

fn parse_manifest_content(content: &str) -> Result<Value, serde_json::Error> {
    serde_json::from_str(content.trim_start_matches('\u{feff}'))
}

pub(crate) fn read_manifest(app: &AppHandle) -> Result<Value, String> {
    let manifest_path = model_root(app)?.join("manifest.json");
    if !manifest_path.exists() {
        return Ok(default_manifest());
    }
    let content = fs::read_to_string(&manifest_path).map_err(|e| {
        format!(
            "failed to read OCR manifest {}: {e}",
            manifest_path.display()
        )
    })?;
    if content.trim_start_matches('\u{feff}').trim().is_empty() {
        let manifest = default_manifest();
        write_manifest(app, &manifest)?;
        return Ok(manifest);
    }
    match parse_manifest_content(&content) {
        Ok(mut manifest) => {
            normalize_manifest_schema(&mut manifest);
            Ok(manifest)
        }
        Err(error) => {
            let backup_path = manifest_path.with_extension(format!(
                "json.broken-{}",
                chrono::Local::now().format("%Y%m%d%H%M%S")
            ));
            fs::copy(&manifest_path, &backup_path).map_err(|copy_error| {
                format!(
                    "failed to parse OCR manifest {}: {error}; failed to backup broken manifest: {copy_error}",
                    manifest_path.display()
                )
            })?;
            let manifest = default_manifest();
            write_manifest(app, &manifest)?;
            Ok(manifest)
        }
    }
}

pub(crate) fn manifest_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(model_root(app)?.join("manifest.json"))
}

pub(crate) fn write_manifest(app: &AppHandle, manifest: &Value) -> Result<(), String> {
    let path = manifest_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create OCR model directory: {e}"))?;
    }
    let temp_path = path.with_extension("json.tmp");
    let body = serde_json::to_string_pretty(manifest)
        .map_err(|e| format!("failed to serialize OCR manifest: {e}"))?;
    fs::write(&temp_path, body).map_err(|e| format!("failed to write OCR manifest: {e}"))?;
    fs::rename(&temp_path, &path).map_err(|e| format!("failed to commit OCR manifest: {e}"))?;
    Ok(())
}

pub(crate) fn normalize_manifest_schema(manifest: &mut Value) {
    let defaults = default_manifest();
    for field in [
        "kind",
        "schemaVersion",
        "runtime",
        "runtimeVersion",
        "modelSetVersion",
        "defaultSourceLanguage",
        "defaultProfile",
        "modelSchema",
        "selfTestSamples",
        "sourcePolicy",
    ] {
        if manifest[field].is_null() {
            manifest[field] = defaults[field].clone();
        }
    }
    if manifest["defaultSourceLanguage"].as_str() != Some("auto") {
        manifest["defaultSourceLanguage"] = json!("auto");
    }
    merge_array_items_by_id(manifest, &defaults, "packs");
    merge_array_items_by_id(manifest, &defaults, "models");
}

fn merge_array_items_by_id(manifest: &mut Value, defaults: &Value, field: &str) {
    if !manifest[field].is_array() {
        manifest[field] = defaults[field].clone();
        return;
    }
    let Some(default_items) = defaults[field].as_array() else {
        return;
    };
    let Some(items) = manifest[field].as_array_mut() else {
        return;
    };
    for default_item in default_items {
        let Some(default_id) = default_item["id"].as_str() else {
            continue;
        };
        if let Some(item) = items
            .iter_mut()
            .find(|item| item["id"].as_str() == Some(default_id))
        {
            merge_missing_object_fields(item, default_item);
        } else {
            items.push(default_item.clone());
        }
    }
}

fn merge_missing_object_fields(target: &mut Value, defaults: &Value) {
    let Some(default_object) = defaults.as_object() else {
        return;
    };
    let Some(target_object) = target.as_object_mut() else {
        return;
    };
    for (key, default_value) in default_object {
        if target_object
            .get(key)
            .map(|value| value.is_null())
            .unwrap_or(true)
        {
            target_object.insert(key.clone(), default_value.clone());
        }
    }
}
pub(crate) fn now_rfc3339() -> String {
    chrono::Local::now().to_rfc3339()
}

pub(crate) fn set_pack_status(
    manifest: &mut Value,
    pack_id: &str,
    status: &str,
    error: Option<&str>,
) -> Result<(), String> {
    let packs = manifest["packs"]
        .as_array_mut()
        .ok_or_else(|| "OCR manifest packs field is invalid".to_string())?;
    let pack = packs
        .iter_mut()
        .find(|pack| pack["id"].as_str() == Some(pack_id))
        .ok_or_else(|| format!("unknown OCR model pack: {pack_id}"))?;
    pack["status"] = json!(status);
    pack["lastTouchedAt"] = json!(now_rfc3339());
    if let Some(message) = error {
        pack["error"] = json!(message);
    } else if let Some(object) = pack.as_object_mut() {
        object.remove("error");
    }
    Ok(())
}

pub(crate) fn write_pack_install_state(
    app: &AppHandle,
    pack_id: &str,
    operation_id: &str,
    status: &str,
    message: &str,
) -> Result<PathBuf, String> {
    let pack_dir = model_root(app)?.join("packs").join(pack_id);
    fs::create_dir_all(&pack_dir)
        .map_err(|e| format!("failed to create OCR pack directory: {e}"))?;
    let state_path = pack_dir.join("install-state.json");
    let state = json!({
        "schemaVersion": 1,
        "packId": pack_id,
        "operationId": operation_id,
        "status": status,
        "message": message,
        "updatedAt": now_rfc3339()
    });
    let body = serde_json::to_string_pretty(&state)
        .map_err(|e| format!("failed to serialize OCR pack state: {e}"))?;
    fs::write(&state_path, body).map_err(|e| format!("failed to write OCR pack state: {e}"))?;
    Ok(pack_dir)
}

pub(crate) fn validate_manifest(manifest: &Value) -> Vec<Value> {
    let mut issues = crate::ysn_ocr_model_schema::validate_manifest_schema(manifest);
    let Some(packs) = manifest["packs"].as_array() else {
        issues.push(json!({ "severity": "error", "code": "packs-missing", "message": "Manifest packs must be an array." }));
        return issues;
    };
    let models = manifest["models"].as_array().cloned().unwrap_or_default();
    let model_ids: std::collections::HashSet<String> = models
        .iter()
        .filter_map(|model| model["id"].as_str().map(|id| id.to_string()))
        .collect();

    for pack in packs {
        let pack_id = pack["id"].as_str().unwrap_or("unknown");
        if pack["required"].as_bool().unwrap_or(false)
            && pack["modelIds"]
                .as_array()
                .map(|ids| ids.is_empty())
                .unwrap_or(true)
        {
            issues.push(json!({ "severity": "error", "code": "required-pack-empty", "packId": pack_id, "message": "Required OCR pack does not declare modelIds." }));
        }
        if let Some(ids) = pack["modelIds"].as_array() {
            for id in ids.iter().filter_map(|value| value.as_str()) {
                if !model_ids.contains(id) {
                    issues.push(json!({ "severity": "error", "code": "model-missing", "packId": pack_id, "modelId": id, "message": "Pack references a missing model descriptor." }));
                }
            }
        }
    }

    for model in &models {
        let model_id = model["id"].as_str().unwrap_or("unknown");
        let path = model["path"].as_str().unwrap_or("");
        if !crate::ysn_ocr_model_downloader::is_safe_relative_model_path(path) {
            issues.push(json!({ "severity": "error", "code": "unsafe-model-path", "modelId": model_id, "message": "Model path must be a safe relative path." }));
        }
        let source_url = model["source"]["url"].as_str().unwrap_or("");
        if source_url.trim().is_empty() {
            issues.push(json!({ "severity": "warning", "code": "source-url-missing", "modelId": model_id, "message": "Model download source is not configured yet." }));
        }
        let sha256 = model["sha256"].as_str().unwrap_or("");
        if !crate::ysn_ocr_model_downloader::is_sha256_hex(sha256) {
            issues.push(json!({ "severity": "warning", "code": "sha256-missing", "modelId": model_id, "message": "Model SHA256 is not configured yet." }));
        }
        if model["type"].as_str() == Some("recognition") {
            let dictionary = &model["contract"]["dictionary"];
            let dictionary_sha256 = dictionary["sha256"].as_str().unwrap_or("");
            if !crate::ysn_ocr_model_downloader::is_sha256_hex(dictionary_sha256) {
                issues.push(json!({ "severity": "warning", "code": "dictionary-sha256-missing", "modelId": model_id, "message": "Recognition dictionary SHA256 is not configured yet." }));
            }
            if dictionary["size"].as_u64().unwrap_or(0) == 0 {
                issues.push(json!({ "severity": "warning", "code": "dictionary-size-missing", "modelId": model_id, "message": "Recognition dictionary size is not configured yet." }));
            }
        }
    }

    issues
}

pub(crate) fn collect_pack_download_plan(
    manifest: &Value,
    pack_id: &str,
) -> Result<Vec<Value>, String> {
    let packs = manifest["packs"]
        .as_array()
        .ok_or_else(|| "OCR manifest packs field is invalid".to_string())?;
    let pack = packs
        .iter()
        .find(|pack| pack["id"].as_str() == Some(pack_id))
        .ok_or_else(|| format!("unknown OCR model pack: {pack_id}"))?;
    let model_ids: std::collections::HashSet<&str> = pack["modelIds"]
        .as_array()
        .ok_or_else(|| format!("OCR model pack has no modelIds: {pack_id}"))?
        .iter()
        .filter_map(|value| value.as_str())
        .collect();
    if model_ids.is_empty() {
        return Err(format!(
            "OCR model pack has no downloadable models: {pack_id}"
        ));
    }

    let models = manifest["models"]
        .as_array()
        .ok_or_else(|| "OCR manifest models field is invalid".to_string())?;
    let mut plan = Vec::new();
    for model_id in model_ids {
        let model = models
            .iter()
            .find(|model| model["id"].as_str() == Some(model_id))
            .ok_or_else(|| format!("OCR model descriptor not found: {model_id}"))?;
        let relative_path = model["path"].as_str().unwrap_or("");
        if !crate::ysn_ocr_model_downloader::is_safe_relative_model_path(relative_path) {
            return Err(format!("OCR model path is unsafe: {model_id}"));
        }
        let url = model["source"]["url"].as_str().unwrap_or("").trim();
        if url.is_empty() {
            return Err(format!(
                "OCR model download source is not configured: {model_id}"
            ));
        }
        let sha256 = model["sha256"].as_str().unwrap_or("");
        if !crate::ysn_ocr_model_downloader::is_sha256_hex(sha256) {
            return Err(format!("OCR model SHA256 is not configured: {model_id}"));
        }
        let size = model["size"].as_u64().unwrap_or(0);
        if size == 0 {
            return Err(format!("OCR model size is not configured: {model_id}"));
        }
        let provider = model["source"]["provider"].as_str().unwrap_or("");
        if provider != "ysn-managed" {
            return Err(format!(
                "OCR model source provider is not production managed: {model_id}"
            ));
        }
        let license = model["source"]["license"].as_str().unwrap_or("");
        if license.is_empty() || license == "pending-review" {
            return Err(format!(
                "OCR model license review is not configured: {model_id}"
            ));
        }
        plan.push(json!({
            "artifactType": "model",
            "modelId": model_id,
            "url": url,
            "sha256": sha256,
            "relativePath": relative_path,
            "size": size,
            "packId": pack_id,
            "provider": provider,
            "license": license,
            "version": model["version"].clone()
        }));
        if model["type"].as_str() == Some("recognition") {
            append_dictionary_download_plan(&mut plan, model, model_id, pack_id)?;
        }
    }
    Ok(plan)
}

fn append_dictionary_download_plan(
    plan: &mut Vec<Value>,
    model: &Value,
    model_id: &str,
    pack_id: &str,
) -> Result<(), String> {
    let dictionary = &model["contract"]["dictionary"];
    let relative_path = dictionary["path"].as_str().unwrap_or("");
    if !crate::ysn_ocr_dictionary::is_safe_dictionary_path(relative_path) {
        return Err(format!("OCR dictionary path is unsafe: {model_id}"));
    }
    let url = dictionary["source"]["url"].as_str().unwrap_or("").trim();
    if url.is_empty() {
        return Err(format!(
            "OCR dictionary download source is not configured: {model_id}"
        ));
    }
    let sha256 = dictionary["sha256"].as_str().unwrap_or("");
    if !crate::ysn_ocr_model_downloader::is_sha256_hex(sha256) {
        return Err(format!(
            "OCR dictionary SHA256 is not configured: {model_id}"
        ));
    }
    let size = dictionary["size"].as_u64().unwrap_or(0);
    if size == 0 {
        return Err(format!("OCR dictionary size is not configured: {model_id}"));
    }
    let provider = dictionary["source"]["provider"].as_str().unwrap_or("");
    if provider != "ysn-managed" {
        return Err(format!(
            "OCR dictionary source provider is not production managed: {model_id}"
        ));
    }
    let license = dictionary["source"]["license"].as_str().unwrap_or("");
    if license.is_empty() || license == "pending-review" {
        return Err(format!(
            "OCR dictionary license review is not configured: {model_id}"
        ));
    }
    plan.push(json!({
        "artifactType": "dictionary",
        "modelId": model_id,
        "script": dictionary["script"].clone(),
        "url": url,
        "sha256": sha256,
        "relativePath": relative_path,
        "size": size,
        "packId": pack_id,
        "provider": provider,
        "license": license,
        "version": model["version"].clone()
    }));
    Ok(())
}

pub(crate) fn set_installed_pack_self_test_time(manifest: &mut Value, tested_at: &str) {
    if let Some(packs) = manifest["packs"].as_array_mut() {
        for pack in packs
            .iter_mut()
            .filter(|pack| pack["status"].as_str() == Some("installed"))
        {
            pack["lastSelfTestAt"] = json!(tested_at);
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn parse_manifest_content_accepts_utf8_bom() {
        let manifest = super::parse_manifest_content(
            "\u{feff}{\"kind\":\"ysn-ocr-runtime-manifest\",\"packs\":[],\"models\":[]}",
        )
        .unwrap();
        assert_eq!(manifest["kind"].as_str(), Some("ysn-ocr-runtime-manifest"));
    }

    #[test]
    fn normalize_manifest_schema_upgrades_legacy_manifest() {
        let mut manifest = json!({
            "runtime": "ysn-ocr-runtime",
            "defaultSourceLanguage": "en",
            "packs": [{
                "id": "auto-multilingual-balanced",
                "required": true,
                "modelIds": ["det-default"]
            }],
            "models": [{
                "id": "det-default",
                "type": "detection",
                "engine": "onnxruntime",
                "path": "models/det-default.onnx"
            }]
        });
        super::normalize_manifest_schema(&mut manifest);
        assert_eq!(
            manifest["kind"].as_str(),
            Some(crate::ysn_ocr_model_schema::MANIFEST_KIND)
        );
        assert_eq!(manifest["defaultSourceLanguage"].as_str(), Some("auto"));
        assert!(manifest["modelSchema"].is_object());
        assert!(manifest["selfTestSamples"]
            .as_array()
            .unwrap()
            .iter()
            .any(|sample| sample["id"] == "latin-ui-technical-path"));
        let det = manifest["models"]
            .as_array()
            .unwrap()
            .iter()
            .find(|model| model["id"] == "det-default")
            .unwrap();
        assert!(det["contract"]["decoder"].is_object());
    }
    fn production_manifest() -> serde_json::Value {
        json!({
            "packs": [{ "id": "core-latin", "required": true, "modelIds": ["latin-rec"] }],
            "models": [{
                "id": "latin-rec",
                "path": "latin/rec.onnx",
                "packId": "core-latin",
                "version": "2026.06.ocr.v1",
                "sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "size": 1024,
                "source": {
                    "provider": "ysn-managed",
                    "url": "https://models.example.invalid/artifacts/2026.06.ocr.v1/core-latin/latin-rec.onnx",
                    "license": "reviewed-commercial"
                }
            }]
        })
    }

    fn production_recognition_manifest() -> serde_json::Value {
        let mut manifest = production_manifest();
        manifest["models"][0]["type"] = json!("recognition");
        manifest["models"][0]["scripts"] = json!(["latin"]);
        manifest["models"][0]["contract"] =
            crate::ysn_ocr_model_schema::recognition_model_contract("latin");
        manifest["models"][0]["contract"]["dictionary"]["sha256"] =
            json!("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc");
        manifest["models"][0]["contract"]["dictionary"]["size"] = json!(2048);
        manifest["models"][0]["contract"]["dictionary"]["source"] = json!({
            "provider": "ysn-managed",
            "url": "https://models.example.invalid/artifacts/2026.06.ocr.v1/core-latin/dictionaries/latin.txt",
            "license": "reviewed-commercial"
        });
        manifest
    }

    #[test]
    fn collect_pack_download_plan_includes_release_metadata() {
        let manifest = production_manifest();
        let plan = super::collect_pack_download_plan(&manifest, "core-latin").unwrap();
        assert_eq!(plan[0]["artifactType"].as_str(), Some("model"));
        assert_eq!(plan[0]["modelId"].as_str(), Some("latin-rec"));
        assert_eq!(plan[0]["provider"].as_str(), Some("ysn-managed"));
        assert_eq!(plan[0]["license"].as_str(), Some("reviewed-commercial"));
        assert_eq!(plan[0]["size"].as_u64(), Some(1024));
        assert_eq!(plan[0]["version"].as_str(), Some("2026.06.ocr.v1"));
    }

    #[test]
    fn collect_pack_download_plan_rejects_unreviewed_license() {
        let mut manifest = production_manifest();
        manifest["models"][0]["source"]["license"] = json!("pending-review");
        let error = super::collect_pack_download_plan(&manifest, "core-latin").unwrap_err();
        assert!(error.contains("license review"));
    }

    #[test]
    fn collect_pack_download_plan_includes_dictionary_artifact() {
        let manifest = production_recognition_manifest();
        let plan = super::collect_pack_download_plan(&manifest, "core-latin").unwrap();

        assert_eq!(plan.len(), 2);
        assert_eq!(plan[1]["artifactType"].as_str(), Some("dictionary"));
        assert_eq!(
            plan[1]["relativePath"].as_str(),
            Some("dictionaries/latin.txt")
        );
        assert_eq!(plan[1]["script"].as_str(), Some("latin"));
    }

    #[test]
    fn collect_pack_download_plan_rejects_missing_dictionary_source() {
        let mut manifest = production_recognition_manifest();
        manifest["models"][0]["contract"]["dictionary"]["source"]["url"] = json!("");

        let error = super::collect_pack_download_plan(&manifest, "core-latin").unwrap_err();

        assert!(error.contains("dictionary download source"));
    }

    #[test]
    fn validate_manifest_reports_missing_dictionary_artifacts() {
        let manifest = crate::ysn_ocr_model_index::default_manifest("1.1.0", "2026.06.ocr.v1");
        let issues = super::validate_manifest(&manifest);

        assert!(issues
            .iter()
            .any(|issue| issue["code"] == "dictionary-sha256-missing"));
        assert!(issues
            .iter()
            .any(|issue| issue["code"] == "dictionary-size-missing"));
    }
}

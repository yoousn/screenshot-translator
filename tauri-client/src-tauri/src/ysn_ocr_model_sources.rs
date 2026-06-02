use serde_json::{json, Value};

const SOURCE_POLICY_VERSION: &str = "2026.06.trusted-source.v1";
const MANAGED_SOURCE_INDEX_KIND: &str = "ysn-ocr-managed-source-index";
const MANAGED_SOURCE_INDEX_SCHEMA_VERSION: u64 = 1;

fn required_source_fields() -> Vec<&'static str> {
    vec![
        "provider", "url", "license", "sha256", "size", "version", "packId", "path",
    ]
}

fn required_artifact_source_fields() -> Vec<&'static str> {
    vec![
        "artifactType",
        "modelId",
        "provider",
        "url",
        "license",
        "sha256",
        "size",
        "version",
        "packId",
        "path",
    ]
}

pub(crate) fn trusted_source_policy() -> Value {
    json!({
        "policyVersion": SOURCE_POLICY_VERSION,
        "productionDownloadProvider": "ysn-managed",
        "allowedProviderTiers": [
            {
                "id": "ysn-managed",
                "label": "YSN managed source",
                "productionDownloadAllowed": true,
                "description": "Model files are downloaded from a YSN-controlled model index/CDN entry with pinned SHA256 and reviewed license metadata."
            },
            {
                "id": "upstream-reference",
                "label": "Official upstream reference",
                "productionDownloadAllowed": false,
                "description": "Official PaddleOCR/RapidOCR or model-hosting reference used for review and reproducible sourcing, not direct runtime download."
            },
            {
                "id": "local-dev",
                "label": "Local development override",
                "productionDownloadAllowed": false,
                "description": "Developer-selected local model files for experiments and validation only."
            }
        ],
        "requiredFields": required_source_fields(),
        "requiredArtifactFields": required_artifact_source_fields(),
        "rules": [
            "Runtime code must not scatter third-party model URLs.",
            "Production downloads must go through the managed model index.",
            "Every production OCR artifact must have artifactType, modelId, URL, SHA256, size, version, license, packId, and safe relative path metadata.",
            "Official upstream sources are recorded as review evidence, not automatically trusted production downloads.",
            "A pack without trusted source metadata is recoverable but not ready."
        ],
        "upstreamReferences": [
            {
                "provider": "PaddleOCR",
                "purpose": "PP-OCRv5 model generation and official model lineage review",
                "status": "reference-only"
            },
            {
                "provider": "RapidOCR",
                "purpose": "ONNX packaging compatibility and lightweight inference reference",
                "status": "reference-only"
            }
        ]
    })
}

fn model_source_issue(model: &Value) -> Option<Value> {
    let model_id = model["id"].as_str().unwrap_or("unknown");
    let provider = model["source"]["provider"].as_str().unwrap_or("").trim();
    let url = model["source"]["url"].as_str().unwrap_or("").trim();
    let license = model["source"]["license"].as_str().unwrap_or("").trim();
    let sha256 = model["sha256"].as_str().unwrap_or("").trim();
    let size = model["size"].as_u64().unwrap_or(0);
    let path = model["path"].as_str().unwrap_or("").trim();
    let version = model["version"].as_str().unwrap_or("").trim();
    let pack_id = model["packId"].as_str().unwrap_or("").trim();

    if provider != "ysn-managed" {
        return Some(json!({
            "severity": "warning",
            "code": "source-provider-not-managed",
            "modelId": model_id,
            "provider": provider,
            "message": "Model source provider is not approved for production downloads."
        }));
    }
    if url.is_empty() {
        return Some(json!({
            "severity": "warning",
            "code": "managed-source-url-missing",
            "modelId": model_id,
            "message": "Managed model source URL is not configured yet."
        }));
    }
    if !(url.starts_with("https://")
        || url.starts_with("http://localhost")
        || url.starts_with("http://127.0.0.1"))
    {
        return Some(json!({
            "severity": "warning",
            "code": "managed-source-url-untrusted",
            "modelId": model_id,
            "message": "Managed model source URL must be HTTPS outside local development."
        }));
    }
    if !crate::ysn_ocr_model_downloader::is_sha256_hex(sha256) {
        return Some(json!({
            "severity": "warning",
            "code": "managed-source-sha256-missing",
            "modelId": model_id,
            "message": "Managed model source SHA256 is not configured yet."
        }));
    }
    if license.is_empty() || license == "pending-review" {
        return Some(json!({
            "severity": "warning",
            "code": "managed-source-license-pending",
            "modelId": model_id,
            "message": "Managed model source license review is not complete."
        }));
    }
    if size == 0 || path.is_empty() || version.is_empty() || pack_id.is_empty() {
        return Some(json!({
            "severity": "warning",
            "code": "managed-source-metadata-incomplete",
            "modelId": model_id,
            "message": "Managed model source metadata is incomplete."
        }));
    }

    None
}

pub(crate) fn managed_source_publish_layout() -> Value {
    json!({
        "kind": "ysn-ocr-managed-source-publish-layout",
        "schemaVersion": MANAGED_SOURCE_INDEX_SCHEMA_VERSION,
        "policyVersion": SOURCE_POLICY_VERSION,
        "root": {
            "index": "index/ysn-ocr-managed-source-index.json",
            "packs": "packs/{packId}/pack.json",
            "artifacts": "artifacts/{modelSetVersion}/{packId}/{modelId}.onnx",
            "dictionaryArtifacts": "artifacts/{modelSetVersion}/{packId}/dictionaries/{script}.txt",
            "licenses": "licenses/{licenseId}.json",
            "checksums": "checksums/{modelSetVersion}.sha256"
        },
        "requiredReleaseFields": [
            "kind",
            "schemaVersion",
            "policyVersion",
            "modelSetVersion",
            "publishedAt",
            "publisher",
            "models"
        ],
        "requiredModelFields": required_source_fields(),
        "requiredArtifactFields": required_artifact_source_fields(),
        "rollback": {
            "strategy": "keep-previous-index-and-active-models-until-new-pack-verifies",
            "requiresPreviousIndex": true
        },
        "readinessRule": "A source index may be imported only after every required OCR artifact has HTTPS/localhost URL, SHA256, size, reviewed license, version, packId, and safe manifest-matching path."
    })
}

fn validate_managed_source_index(index: &Value, manifest: &Value) -> Result<(), String> {
    if index["kind"].as_str().unwrap_or(MANAGED_SOURCE_INDEX_KIND) != MANAGED_SOURCE_INDEX_KIND {
        return Err("Managed OCR source index kind is invalid.".to_string());
    }
    if index["schemaVersion"]
        .as_u64()
        .unwrap_or(MANAGED_SOURCE_INDEX_SCHEMA_VERSION)
        != MANAGED_SOURCE_INDEX_SCHEMA_VERSION
    {
        return Err("Managed OCR source index schemaVersion is unsupported.".to_string());
    }
    let policy_version = index["policyVersion"]
        .as_str()
        .unwrap_or(SOURCE_POLICY_VERSION);
    if policy_version != SOURCE_POLICY_VERSION {
        return Err(format!(
            "Managed OCR source index policyVersion is unsupported: {policy_version}"
        ));
    }
    let manifest_model_set = manifest["modelSetVersion"].as_str().unwrap_or("");
    let index_model_set = index["modelSetVersion"]
        .as_str()
        .unwrap_or(manifest_model_set);
    if !manifest_model_set.is_empty() && index_model_set != manifest_model_set {
        return Err(format!(
            "Managed OCR source index modelSetVersion does not match manifest: {index_model_set}"
        ));
    }
    if let Some(layout) = index.get("publishLayout") {
        let layout_kind = layout["kind"].as_str().unwrap_or("");
        if layout_kind != "ysn-ocr-managed-source-publish-layout" {
            return Err("Managed OCR source index publishLayout kind is invalid.".to_string());
        }
    }
    Ok(())
}
pub(crate) fn source_readiness(manifest: &Value) -> Value {
    let models = manifest["models"].as_array().cloned().unwrap_or_default();
    let required_models: Vec<Value> = models
        .into_iter()
        .filter(|model| model["required"].as_bool().unwrap_or(false))
        .collect();
    let required_count = required_models.len();
    let mut issues = Vec::new();
    let mut configured_models = Vec::new();
    let mut pending_models = Vec::new();

    for model in required_models {
        let model_id = model["id"].as_str().unwrap_or("unknown").to_string();
        if let Some(issue) = model_source_issue(&model) {
            pending_models.push(model_id);
            issues.push(issue);
        } else {
            configured_models.push(model_id);
        }
    }

    json!({
        "ready": required_count > 0 && issues.is_empty(),
        "requiredModels": required_count,
        "configuredModels": configured_models.len(),
        "configuredModelIds": configured_models,
        "pendingModelIds": pending_models,
        "issues": issues,
        "policy": trusted_source_policy(),
        "nextAction": if required_count == 0 { "define-required-models" } else if issues.is_empty() { "install-or-self-test-model-packs" } else { "configure-managed-model-sources" }
    })
}

pub(crate) fn pack_source_blocker(manifest: &Value, pack_id: &str) -> Option<String> {
    let packs = manifest["packs"].as_array()?;
    let pack = packs
        .iter()
        .find(|pack| pack["id"].as_str() == Some(pack_id))?;
    let model_ids: std::collections::HashSet<String> = pack["modelIds"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|value| value.as_str().map(|id| id.to_string()))
        .collect();
    if model_ids.is_empty() {
        return Some("OCR model pack has no model IDs to install.".to_string());
    }

    let readiness = source_readiness(manifest);
    let issues = readiness["issues"].as_array().cloned().unwrap_or_default();
    for issue in issues {
        let model_id = issue["modelId"].as_str().unwrap_or("");
        if model_ids.contains(model_id) {
            return Some(
                issue["message"]
                    .as_str()
                    .unwrap_or("Trusted model source is not configured.")
                    .to_string(),
            );
        }
    }
    None
}

fn source_entry_field<'a>(entry: &'a Value, key: &str) -> &'a str {
    entry[key]
        .as_str()
        .or_else(|| entry["source"][key].as_str())
        .unwrap_or("")
        .trim()
}

fn source_entry_size(entry: &Value) -> u64 {
    entry["size"]
        .as_u64()
        .or_else(|| entry["source"]["size"].as_u64())
        .unwrap_or(0)
}

fn legacy_model_entries(index: &Value) -> Result<Vec<Value>, String> {
    match index.get("models") {
        Some(models) => models
            .as_array()
            .cloned()
            .ok_or_else(|| "Managed OCR source index models field must be an array.".to_string()),
        None => Ok(Vec::new()),
    }
}

fn artifact_entries(index: &Value) -> Result<Vec<Value>, String> {
    let mut entries = legacy_model_entries(index)?
        .into_iter()
        .map(|mut entry| {
            entry["artifactType"] = json!("model");
            if entry.get("modelId").is_none() {
                entry["modelId"] = entry["id"].clone();
            }
            entry
        })
        .collect::<Vec<_>>();

    if let Some(artifacts) = index.get("artifacts") {
        let artifacts = artifacts.as_array().ok_or_else(|| {
            "Managed OCR source index artifacts field must be an array.".to_string()
        })?;
        entries.extend(artifacts.iter().cloned());
    }

    if entries.is_empty() {
        return Err(
            "Managed OCR source index does not contain any model or artifact entries.".to_string(),
        );
    }

    Ok(entries)
}

fn validate_common_artifact_metadata<'a>(
    entry: &'a Value,
    model_id: &str,
) -> Result<
    (
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        u64,
    ),
    String,
> {
    let provider = source_entry_field(entry, "provider");
    let url = source_entry_field(entry, "url");
    let license = source_entry_field(entry, "license");
    let sha256 = source_entry_field(entry, "sha256");
    let version = source_entry_field(entry, "version");
    let path = source_entry_field(entry, "path");
    let pack_id = source_entry_field(entry, "packId");
    let size = source_entry_size(entry);

    if provider != "ysn-managed" {
        return Err(format!(
            "Managed source entry must use provider ysn-managed: {model_id}"
        ));
    }
    if !(url.starts_with("https://")
        || url.starts_with("http://localhost")
        || url.starts_with("http://127.0.0.1"))
    {
        return Err(format!(
            "Managed source URL must be HTTPS outside local development: {model_id}"
        ));
    }
    if !crate::ysn_ocr_model_downloader::is_sha256_hex(sha256) {
        return Err(format!("Managed source SHA256 is invalid: {model_id}"));
    }
    if size == 0 {
        return Err(format!("Managed source size is required: {model_id}"));
    }
    if license.is_empty() || license == "pending-review" {
        return Err(format!(
            "Managed source license review is required: {model_id}"
        ));
    }
    if version.is_empty() || path.is_empty() || pack_id.is_empty() {
        return Err(format!("Managed source metadata is incomplete: {model_id}"));
    }

    Ok((provider, url, license, sha256, version, path, pack_id, size))
}

pub(crate) fn apply_managed_source_index(
    manifest: &mut Value,
    index: &Value,
) -> Result<Value, String> {
    validate_managed_source_index(index, manifest)?;
    let entries = artifact_entries(index)?;

    let models = manifest["models"]
        .as_array_mut()
        .ok_or_else(|| "OCR manifest models field is invalid.".to_string())?;
    let mut updated_models = Vec::new();
    let mut updated_artifacts = Vec::new();

    for entry in &entries {
        let artifact_type = entry["artifactType"].as_str().unwrap_or("model").trim();
        let model_id = entry["modelId"]
            .as_str()
            .or_else(|| entry["id"].as_str())
            .ok_or_else(|| "Managed OCR source index entry is missing modelId.".to_string())?
            .trim();
        let (_provider, url, license, sha256, version, path, pack_id, size) =
            validate_common_artifact_metadata(entry, model_id)?;

        let model = models
            .iter_mut()
            .find(|model| model["id"].as_str() == Some(model_id))
            .ok_or_else(|| {
                format!("Managed source entry references unknown model id: {model_id}")
            })?;
        let manifest_pack_id = model["packId"].as_str().unwrap_or("");
        if manifest_pack_id != pack_id {
            return Err(format!(
                "Managed source packId does not match manifest descriptor: {model_id}"
            ));
        }

        match artifact_type {
            "model" => {
                if !crate::ysn_ocr_model_downloader::is_safe_relative_model_path(path) {
                    return Err(format!("Managed source path is unsafe: {model_id}"));
                }
                let manifest_path = model["path"].as_str().unwrap_or("");
                if manifest_path != path {
                    return Err(format!(
                        "Managed source path does not match manifest descriptor: {model_id}"
                    ));
                }
                model["source"] = json!({
                    "provider": "ysn-managed",
                    "url": url,
                    "license": license,
                });
                model["sha256"] = json!(sha256);
                model["size"] = json!(size);
                model["version"] = json!(version);
                updated_models.push(model_id.to_string());
                updated_artifacts.push(format!("{model_id}:model"));
            }
            "dictionary" => {
                if !crate::ysn_ocr_dictionary::is_safe_dictionary_path(path) {
                    return Err(format!("Managed dictionary path is unsafe: {model_id}"));
                }
                if model["type"].as_str() != Some("recognition") {
                    return Err(format!(
                        "Managed dictionary artifact requires recognition model: {model_id}"
                    ));
                }
                let dictionary = &mut model["contract"]["dictionary"];
                let manifest_path = dictionary["path"].as_str().unwrap_or("");
                if manifest_path != path {
                    return Err(format!(
                        "Managed dictionary path does not match manifest descriptor: {model_id}"
                    ));
                }
                if let Some(script) = entry["script"].as_str() {
                    let script = script.trim();
                    let manifest_script = dictionary["script"].as_str().unwrap_or("");
                    if !script.is_empty() && manifest_script != script {
                        return Err(format!(
                            "Managed dictionary script does not match manifest descriptor: {model_id}"
                        ));
                    }
                }
                dictionary["source"] = json!({
                    "provider": "ysn-managed",
                    "url": url,
                    "license": license,
                    "version": version,
                    "packId": pack_id,
                });
                dictionary["sha256"] = json!(sha256);
                dictionary["size"] = json!(size);
                updated_artifacts.push(format!("{model_id}:dictionary"));
            }
            other => {
                return Err(format!(
                    "Managed OCR source artifactType is unsupported for {model_id}: {other}"
                ));
            }
        }
    }

    manifest["managedSourceIndex"] = json!({
        "importedAt": crate::ysn_ocr_manifest_store::now_rfc3339(),
        "updatedModelIds": updated_models,
        "updatedArtifacts": updated_artifacts,
        "policyVersion": SOURCE_POLICY_VERSION,
    });

    Ok(json!({
        "ok": true,
        "updatedModels": manifest["managedSourceIndex"]["updatedModelIds"].clone(),
        "updatedArtifacts": manifest["managedSourceIndex"]["updatedArtifacts"].clone(),
        "updatedCount": manifest["managedSourceIndex"]["updatedModelIds"].as_array().map(|items| items.len()).unwrap_or(0),
        "updatedArtifactCount": manifest["managedSourceIndex"]["updatedArtifacts"].as_array().map(|items| items.len()).unwrap_or(0),
        "sourceReadiness": source_readiness(manifest),
    }))
}
pub(crate) fn dry_run_managed_source_index(
    manifest: &Value,
    index: &Value,
    pack_id: Option<&str>,
) -> Result<Value, String> {
    let mut candidate = manifest.clone();
    let import_result = apply_managed_source_index(&mut candidate, index)?;
    let packs = candidate["packs"].as_array().cloned().unwrap_or_default();
    let target_pack_ids: Vec<String> = packs
        .iter()
        .filter(|pack| {
            if let Some(target) = pack_id {
                pack["id"].as_str() == Some(target)
            } else {
                pack["required"].as_bool().unwrap_or(false)
            }
        })
        .filter_map(|pack| pack["id"].as_str().map(|id| id.to_string()))
        .collect();
    if target_pack_ids.is_empty() {
        return Err(pack_id
            .map(|id| format!("unknown OCR model pack: {id}"))
            .unwrap_or_else(|| "No required OCR model packs are defined.".to_string()));
    }

    let mut pack_plans = Vec::new();
    for target_pack_id in target_pack_ids {
        if let Some(blocker) = pack_source_blocker(&candidate, &target_pack_id) {
            pack_plans.push(json!({
                "packId": target_pack_id,
                "ok": false,
                "blocker": blocker,
                "downloadPlan": []
            }));
            continue;
        }
        match crate::ysn_ocr_manifest_store::collect_pack_download_plan(&candidate, &target_pack_id)
        {
            Ok(plan) => pack_plans.push(json!({
                "packId": target_pack_id,
                "ok": true,
                "blocker": null,
                "downloadPlan": plan,
                "modelCount": plan.len()
            })),
            Err(error) => pack_plans.push(json!({
                "packId": target_pack_id,
                "ok": false,
                "blocker": error,
                "downloadPlan": []
            })),
        }
    }
    let ok = pack_plans
        .iter()
        .all(|plan| plan["ok"].as_bool().unwrap_or(false));
    Ok(json!({
        "ok": ok,
        "mode": "dry-run",
        "wouldWriteManifest": false,
        "wouldActivateModels": false,
        "importResult": import_result,
        "sourceReadiness": source_readiness(&candidate),
        "packPlans": pack_plans,
        "publishLayout": index.get("publishLayout").cloned().unwrap_or_else(managed_source_publish_layout)
    }))
}
pub(crate) fn managed_source_index_template(manifest: &Value) -> Value {
    let models = manifest["models"].as_array().cloned().unwrap_or_default();
    let template_models: Vec<Value> = models
        .iter()
        .filter(|model| model["required"].as_bool().unwrap_or(false))
        .map(|model| json!({
            "id": model["id"].as_str().unwrap_or(""),
            "provider": "ysn-managed",
            "url": "",
            "license": "pending-review",
            "sha256": "",
            "size": 0,
            "version": model["version"].as_str().unwrap_or(""),
            "packId": model["packId"].as_str().unwrap_or(""),
            "path": model["path"].as_str().unwrap_or(""),
            "notes": "Fill this legacy model entry only after source URL, SHA256, size, version, path, packId, and license metadata have been reviewed. New integrations should prefer the artifacts array."
        }))
        .collect();
    let mut template_artifacts = Vec::new();
    for model in models
        .iter()
        .filter(|model| model["required"].as_bool().unwrap_or(false))
    {
        let model_id = model["id"].as_str().unwrap_or("");
        let pack_id = model["packId"].as_str().unwrap_or("");
        let version = model["version"].as_str().unwrap_or("");
        template_artifacts.push(json!({
            "artifactType": "model",
            "modelId": model_id,
            "provider": "ysn-managed",
            "url": "",
            "license": "pending-review",
            "sha256": "",
            "size": 0,
            "version": version,
            "packId": pack_id,
            "path": model["path"].as_str().unwrap_or(""),
            "notes": "Model artifact metadata. Kept for explicit artifact-level source review."
        }));
        if model["type"].as_str() == Some("recognition") {
            let dictionary = &model["contract"]["dictionary"];
            template_artifacts.push(json!({
                "artifactType": "dictionary",
                "modelId": model_id,
                "script": dictionary["script"].as_str().unwrap_or(""),
                "provider": "ysn-managed",
                "url": "",
                "license": "pending-review",
                "sha256": dictionary["sha256"].as_str().unwrap_or(""),
                "size": dictionary["size"].as_u64().unwrap_or(0),
                "version": version,
                "packId": pack_id,
                "path": dictionary["path"].as_str().unwrap_or(""),
                "notes": "Recognition dictionary artifact metadata. Required before recognition model can be production-ready."
            }));
        }
    }

    json!({
        "kind": MANAGED_SOURCE_INDEX_KIND,
        "schemaVersion": MANAGED_SOURCE_INDEX_SCHEMA_VERSION,
        "policyVersion": SOURCE_POLICY_VERSION,
        "modelSetVersion": manifest["modelSetVersion"].as_str().unwrap_or(""),
        "publishedAt": "",
        "publisher": { "name": "YSN", "contact": "", "signature": "pending" },
        "publishLayout": managed_source_publish_layout(),
        "rules": [
            "Do not add arbitrary third-party URLs directly to application code.",
            "Production downloads must use provider ysn-managed.",
            "Every artifact entry must include reviewed license metadata, HTTPS URL, SHA256, size, version, path, and packId.",
            "path and packId must match the bundled manifest descriptor."
        ],
        "models": template_models,
        "artifacts": template_artifacts
    })
}
#[cfg(test)]
mod tests {
    use serde_json::json;
    use sha2::{Digest, Sha256};
    use std::fs;

    fn sha256_hex(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        format!("{:x}", hasher.finalize())
    }

    fn managed_model() -> serde_json::Value {
        json!({
            "id": "latin-rec",
            "required": true,
            "path": "latin/rec.onnx",
            "version": "1.0.0",
            "packId": "core-latin",
            "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "size": 128,
            "source": {
                "provider": "ysn-managed",
                "url": "https://models.example.invalid/latin/rec.onnx",
                "license": "reviewed-commercial"
            }
        })
    }

    fn managed_recognition_model() -> serde_json::Value {
        let mut model = managed_model();
        model["type"] = json!("recognition");
        model["contract"] = crate::ysn_ocr_model_schema::recognition_model_contract("latin");
        model["contract"]["dictionary"]["source"] = json!({
            "provider": "ysn-managed",
            "url": "https://models.example.invalid/artifacts/2026.06.ocr.v1/core-latin/dictionaries/latin.txt",
            "license": "reviewed-commercial",
            "version": "1.0.0",
            "packId": "core-latin"
        });
        model["contract"]["dictionary"]["sha256"] =
            json!("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc");
        model["contract"]["dictionary"]["size"] = json!(64);
        model
    }

    #[test]
    fn test_source_readiness_accepts_complete_managed_source() {
        let manifest = json!({ "models": [managed_model()] });
        let readiness = super::source_readiness(&manifest);
        assert_eq!(readiness["ready"].as_bool(), Some(true));
        assert_eq!(readiness["requiredModels"].as_u64(), Some(1));
        assert_eq!(readiness["configuredModels"].as_u64(), Some(1));
        assert!(readiness["issues"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_source_readiness_rejects_local_dev_for_production() {
        let mut model = managed_model();
        model["source"]["provider"] = json!("local-dev");
        let manifest = json!({ "models": [model] });
        let readiness = super::source_readiness(&manifest);
        assert_eq!(readiness["ready"].as_bool(), Some(false));
        assert_eq!(readiness["configuredModels"].as_u64(), Some(0));
        assert_eq!(readiness["pendingModelIds"][0].as_str(), Some("latin-rec"));
        assert_eq!(
            readiness["issues"][0]["code"].as_str(),
            Some("source-provider-not-managed")
        );
    }

    #[test]
    fn test_pack_source_blocker_reports_pack_model_issue() {
        let mut model = managed_model();
        model["sha256"] = json!("pending");
        let manifest = json!({
            "models": [model],
            "packs": [{ "id": "core-latin", "modelIds": ["latin-rec"] }]
        });
        let blocker = super::pack_source_blocker(&manifest, "core-latin").unwrap();
        assert!(blocker.contains("SHA256"));
    }

    #[test]
    fn test_pack_source_blocker_allows_complete_managed_pack() {
        let manifest = json!({
            "models": [managed_model()],
            "packs": [{ "id": "core-latin", "modelIds": ["latin-rec"] }]
        });
        assert!(super::pack_source_blocker(&manifest, "core-latin").is_none());
    }

    #[test]
    fn test_apply_managed_source_index_updates_known_model() {
        let mut model = managed_model();
        model["source"] =
            json!({ "provider": "ysn-managed", "url": "", "license": "pending-review" });
        model["sha256"] = json!("");
        model["size"] = json!(0);
        let mut manifest = json!({ "models": [model] });
        let index = json!({
            "models": [{
                "id": "latin-rec",
                "provider": "ysn-managed",
                "url": "https://models.example.invalid/latin/rec.onnx",
                "license": "reviewed-commercial",
                "sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "size": 256,
                "version": "1.0.1",
                "packId": "core-latin",
                "path": "latin/rec.onnx"
            }]
        });
        let result = super::apply_managed_source_index(&mut manifest, &index).unwrap();
        assert_eq!(result["updatedCount"].as_u64(), Some(1));
        assert_eq!(
            manifest["models"][0]["sha256"].as_str(),
            Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
        );
        assert_eq!(
            manifest["models"][0]["source"]["license"].as_str(),
            Some("reviewed-commercial")
        );
    }

    #[test]
    fn test_apply_managed_source_index_updates_dictionary_artifact() {
        let mut model = managed_recognition_model();
        model["contract"]["dictionary"]["source"] =
            json!({ "provider": "ysn-managed", "url": "", "license": "pending-review" });
        model["contract"]["dictionary"]["sha256"] = json!("");
        model["contract"]["dictionary"]["size"] = json!(0);
        let mut manifest = json!({ "models": [model] });
        let index = json!({
            "artifacts": [{
                "artifactType": "dictionary",
                "modelId": "latin-rec",
                "script": "latin",
                "provider": "ysn-managed",
                "url": "https://models.example.invalid/artifacts/2026.06.ocr.v1/core-latin/dictionaries/latin.txt",
                "license": "reviewed-commercial",
                "sha256": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
                "size": 512,
                "version": "1.0.1",
                "packId": "core-latin",
                "path": "dictionaries/latin.txt"
            }]
        });

        let result = super::apply_managed_source_index(&mut manifest, &index).unwrap();

        assert_eq!(result["updatedArtifactCount"].as_u64(), Some(1));
        assert_eq!(
            result["updatedArtifacts"][0].as_str(),
            Some("latin-rec:dictionary")
        );
        assert_eq!(
            manifest["models"][0]["contract"]["dictionary"]["sha256"].as_str(),
            Some("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd")
        );
        assert_eq!(
            manifest["models"][0]["contract"]["dictionary"]["source"]["version"].as_str(),
            Some("1.0.1")
        );
    }

    #[test]
    fn test_apply_managed_source_index_rejects_dictionary_path_mismatch() {
        let mut manifest = json!({ "models": [managed_recognition_model()] });
        let index = json!({
            "artifacts": [{
                "artifactType": "dictionary",
                "modelId": "latin-rec",
                "script": "latin",
                "provider": "ysn-managed",
                "url": "https://models.example.invalid/artifacts/2026.06.ocr.v1/core-latin/dictionaries/other.txt",
                "license": "reviewed-commercial",
                "sha256": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
                "size": 512,
                "version": "1.0.1",
                "packId": "core-latin",
                "path": "dictionaries/other.txt"
            }]
        });

        let error = super::apply_managed_source_index(&mut manifest, &index).unwrap_err();

        assert!(error.contains("dictionary path"));
    }

    #[test]
    fn test_apply_managed_source_index_rejects_unsafe_dictionary_path() {
        let mut manifest = json!({ "models": [managed_recognition_model()] });
        let index = json!({
            "artifacts": [{
                "artifactType": "dictionary",
                "modelId": "latin-rec",
                "script": "latin",
                "provider": "ysn-managed",
                "url": "https://models.example.invalid/artifacts/2026.06.ocr.v1/core-latin/dictionaries/latin.txt",
                "license": "reviewed-commercial",
                "sha256": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
                "size": 512,
                "version": "1.0.1",
                "packId": "core-latin",
                "path": "../dictionaries/latin.txt"
            }]
        });

        let error = super::apply_managed_source_index(&mut manifest, &index).unwrap_err();

        assert!(error.contains("unsafe"));
    }

    #[test]
    fn test_apply_managed_source_index_rejects_path_mismatch() {
        let mut manifest = json!({ "models": [managed_model()] });
        let index = json!({
            "models": [{
                "id": "latin-rec",
                "provider": "ysn-managed",
                "url": "https://models.example.invalid/latin/rec.onnx",
                "license": "reviewed-commercial",
                "sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "size": 256,
                "version": "1.0.1",
                "packId": "core-latin",
                "path": "../unsafe.onnx"
            }]
        });
        let error = super::apply_managed_source_index(&mut manifest, &index).unwrap_err();
        assert!(error.contains("unsafe"));
    }

    #[test]
    fn test_managed_source_index_template_matches_manifest_descriptors() {
        let manifest = json!({
            "modelSetVersion": "2026.06.ocr.v1",
            "models": [managed_model()]
        });
        let template = super::managed_source_index_template(&manifest);
        assert_eq!(
            template["kind"].as_str(),
            Some("ysn-ocr-managed-source-index")
        );
        assert_eq!(template["models"][0]["id"].as_str(), Some("latin-rec"));
        assert_eq!(
            template["models"][0]["path"].as_str(),
            Some("latin/rec.onnx")
        );
        assert_eq!(template["models"][0]["packId"].as_str(), Some("core-latin"));
        assert_eq!(template["models"][0]["url"].as_str(), Some(""));
        assert_eq!(
            template["models"][0]["license"].as_str(),
            Some("pending-review")
        );
    }
    #[test]
    fn test_managed_source_index_template_declares_dictionary_artifact() {
        let manifest = json!({
            "modelSetVersion": "2026.06.ocr.v1",
            "models": [managed_recognition_model()]
        });
        let template = super::managed_source_index_template(&manifest);
        let artifacts = template["artifacts"].as_array().unwrap();
        assert_eq!(artifacts.len(), 2);
        assert_eq!(artifacts[0]["artifactType"].as_str(), Some("model"));
        assert_eq!(artifacts[1]["artifactType"].as_str(), Some("dictionary"));
        assert_eq!(artifacts[1]["script"].as_str(), Some("latin"));
        assert_eq!(
            artifacts[1]["path"].as_str(),
            Some("dictionaries/latin.txt")
        );
    }

    #[test]
    fn test_managed_source_template_declares_publish_layout() {
        let manifest = json!({
            "modelSetVersion": "2026.06.ocr.v1",
            "models": [managed_model()]
        });
        let template = super::managed_source_index_template(&manifest);
        assert_eq!(
            template["publishLayout"]["kind"].as_str(),
            Some("ysn-ocr-managed-source-publish-layout")
        );
        assert!(template["publishLayout"]["root"]["artifacts"]
            .as_str()
            .unwrap()
            .contains("{modelSetVersion}"));
        assert_eq!(template["publishedAt"].as_str(), Some(""));
    }

    #[test]
    fn test_apply_managed_source_index_rejects_model_set_mismatch() {
        let mut manifest = json!({
            "modelSetVersion": "2026.06.ocr.v1",
            "models": [managed_model()]
        });
        let index = json!({
            "kind": "ysn-ocr-managed-source-index",
            "schemaVersion": 1,
            "policyVersion": "2026.06.trusted-source.v1",
            "modelSetVersion": "2026.07.ocr.v1",
            "models": [{
                "id": "latin-rec",
                "provider": "ysn-managed",
                "url": "https://models.example.invalid/latin/rec.onnx",
                "license": "reviewed-commercial",
                "sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "size": 256,
                "version": "1.0.1",
                "packId": "core-latin",
                "path": "latin/rec.onnx"
            }]
        });
        let error = super::apply_managed_source_index(&mut manifest, &index).unwrap_err();
        assert!(error.contains("modelSetVersion"));
    }
    #[test]
    fn test_dry_run_managed_source_index_returns_pack_plan() {
        let mut model = managed_model();
        model["source"] =
            json!({ "provider": "ysn-managed", "url": "", "license": "pending-review" });
        model["sha256"] = json!("");
        model["size"] = json!(0);
        let manifest = json!({
            "modelSetVersion": "2026.06.ocr.v1",
            "models": [model],
            "packs": [{ "id": "core-latin", "required": true, "modelIds": ["latin-rec"] }]
        });
        let index = json!({
            "kind": "ysn-ocr-managed-source-index",
            "schemaVersion": 1,
            "policyVersion": "2026.06.trusted-source.v1",
            "modelSetVersion": "2026.06.ocr.v1",
            "publishLayout": super::managed_source_publish_layout(),
            "models": [{
                "id": "latin-rec",
                "provider": "ysn-managed",
                "url": "https://models.example.invalid/artifacts/2026.06.ocr.v1/core-latin/latin-rec.onnx",
                "license": "reviewed-commercial",
                "sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "size": 256,
                "version": "2026.06.ocr.v1",
                "packId": "core-latin",
                "path": "latin/rec.onnx"
            }]
        });
        let result =
            super::dry_run_managed_source_index(&manifest, &index, Some("core-latin")).unwrap();
        assert_eq!(result["ok"].as_bool(), Some(true));
        assert_eq!(result["wouldWriteManifest"].as_bool(), Some(false));
        assert_eq!(result["wouldActivateModels"].as_bool(), Some(false));
        assert_eq!(
            result["packPlans"][0]["downloadPlan"][0]["provider"].as_str(),
            Some("ysn-managed")
        );
    }

    #[test]
    fn test_dry_run_managed_source_index_returns_dictionary_artifact_plan() {
        let mut model = managed_recognition_model();
        model["source"] =
            json!({ "provider": "ysn-managed", "url": "", "license": "pending-review" });
        model["sha256"] = json!("");
        model["size"] = json!(0);
        model["contract"]["dictionary"]["source"] =
            json!({ "provider": "ysn-managed", "url": "", "license": "pending-review" });
        model["contract"]["dictionary"]["sha256"] = json!("");
        model["contract"]["dictionary"]["size"] = json!(0);
        let manifest = json!({
            "modelSetVersion": "2026.06.ocr.v1",
            "models": [model],
            "packs": [{ "id": "core-latin", "required": true, "modelIds": ["latin-rec"] }]
        });
        let index = json!({
            "kind": "ysn-ocr-managed-source-index",
            "schemaVersion": 1,
            "policyVersion": "2026.06.trusted-source.v1",
            "modelSetVersion": "2026.06.ocr.v1",
            "artifacts": [
                {
                    "artifactType": "model",
                    "modelId": "latin-rec",
                    "provider": "ysn-managed",
                    "url": "https://models.example.invalid/artifacts/2026.06.ocr.v1/core-latin/latin-rec.onnx",
                    "license": "reviewed-commercial",
                    "sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                    "size": 256,
                    "version": "2026.06.ocr.v1",
                    "packId": "core-latin",
                    "path": "latin/rec.onnx"
                },
                {
                    "artifactType": "dictionary",
                    "modelId": "latin-rec",
                    "script": "latin",
                    "provider": "ysn-managed",
                    "url": "https://models.example.invalid/artifacts/2026.06.ocr.v1/core-latin/dictionaries/latin.txt",
                    "license": "reviewed-commercial",
                    "sha256": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
                    "size": 512,
                    "version": "2026.06.ocr.v1",
                    "packId": "core-latin",
                    "path": "dictionaries/latin.txt"
                }
            ]
        });

        let result =
            super::dry_run_managed_source_index(&manifest, &index, Some("core-latin")).unwrap();

        assert_eq!(result["ok"].as_bool(), Some(true));
        assert_eq!(
            result["importResult"]["updatedArtifactCount"].as_u64(),
            Some(2)
        );
        assert_eq!(
            result["packPlans"][0]["downloadPlan"][0]["artifactType"].as_str(),
            Some("model")
        );
        assert_eq!(
            result["packPlans"][0]["downloadPlan"][1]["artifactType"].as_str(),
            Some("dictionary")
        );
    }

    #[test]
    fn test_local_managed_source_fixture_smoke_activates_model_and_dictionary() {
        let model_bytes = b"ysn-local-fixture-model-artifact";
        let dictionary_bytes = b"ctc-blank\nA\nB\nC\n";
        let model_sha = sha256_hex(model_bytes);
        let dictionary_sha = sha256_hex(dictionary_bytes);
        let mut model = managed_recognition_model();
        model["source"] =
            json!({ "provider": "ysn-managed", "url": "", "license": "pending-review" });
        model["sha256"] = json!("");
        model["size"] = json!(0);
        model["contract"]["dictionary"]["source"] =
            json!({ "provider": "ysn-managed", "url": "", "license": "pending-review" });
        model["contract"]["dictionary"]["sha256"] = json!("");
        model["contract"]["dictionary"]["size"] = json!(0);
        let manifest = json!({
            "modelSetVersion": "2026.06.ocr.v1",
            "models": [model],
            "packs": [{ "id": "core-latin", "required": true, "modelIds": ["latin-rec"] }]
        });
        let index = json!({
            "kind": "ysn-ocr-managed-source-index",
            "schemaVersion": 1,
            "policyVersion": "2026.06.trusted-source.v1",
            "modelSetVersion": "2026.06.ocr.v1",
            "artifacts": [
                {
                    "artifactType": "model",
                    "modelId": "latin-rec",
                    "provider": "ysn-managed",
                    "url": "https://models.example.invalid/artifacts/2026.06.ocr.v1/core-latin/latin-rec.onnx",
                    "license": "reviewed-commercial",
                    "sha256": model_sha,
                    "size": model_bytes.len(),
                    "version": "2026.06.ocr.v1",
                    "packId": "core-latin",
                    "path": "latin/rec.onnx"
                },
                {
                    "artifactType": "dictionary",
                    "modelId": "latin-rec",
                    "script": "latin",
                    "provider": "ysn-managed",
                    "url": "https://models.example.invalid/artifacts/2026.06.ocr.v1/core-latin/dictionaries/latin.txt",
                    "license": "reviewed-commercial",
                    "sha256": dictionary_sha,
                    "size": dictionary_bytes.len(),
                    "version": "2026.06.ocr.v1",
                    "packId": "core-latin",
                    "path": "dictionaries/latin.txt"
                }
            ]
        });
        let result =
            super::dry_run_managed_source_index(&manifest, &index, Some("core-latin")).unwrap();
        let download_plan = result["packPlans"][0]["downloadPlan"].as_array().unwrap();
        assert_eq!(download_plan.len(), 2);

        let root = std::env::temp_dir().join(format!(
            "ysn-ocr-source-smoke-{}",
            chrono::Local::now()
                .timestamp_nanos_opt()
                .unwrap_or_default()
        ));
        let downloads_root = root.join("downloads");
        let active_root = root.join("active");
        fs::create_dir_all(&downloads_root).unwrap();

        for plan in download_plan {
            let artifact_plan =
                crate::ysn_ocr_model_downloader::parse_artifact_download_plan(plan).unwrap();
            let bytes: &[u8] = match artifact_plan.artifact_type.as_str() {
                "model" => model_bytes,
                "dictionary" => dictionary_bytes,
                other => panic!("unexpected artifact type in smoke fixture: {other}"),
            };
            let download_path = downloads_root.join(format!(
                "{}-{}.download",
                artifact_plan.artifact_type, artifact_plan.model_id
            ));
            fs::write(&download_path, bytes).unwrap();
            let active_path =
                crate::ysn_ocr_model_downloader::verify_and_activate_downloaded_artifact(
                    &download_path,
                    &active_root,
                    &artifact_plan,
                )
                .unwrap();
            assert!(active_path.exists());
            assert_eq!(fs::read(active_path).unwrap(), bytes);
        }

        assert!(active_root.join("latin/rec.onnx").exists());
        assert!(active_root.join("dictionaries/latin.txt").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_dry_run_managed_source_index_reports_unknown_pack() {
        let manifest = json!({
            "modelSetVersion": "2026.06.ocr.v1",
            "models": [managed_model()],
            "packs": [{ "id": "core-latin", "required": true, "modelIds": ["latin-rec"] }]
        });
        let index = json!({
            "kind": "ysn-ocr-managed-source-index",
            "schemaVersion": 1,
            "policyVersion": "2026.06.trusted-source.v1",
            "modelSetVersion": "2026.06.ocr.v1",
            "models": [{
                "id": "latin-rec",
                "provider": "ysn-managed",
                "url": "https://models.example.invalid/latin/rec.onnx",
                "license": "reviewed-commercial",
                "sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "size": 256,
                "version": "2026.06.ocr.v1",
                "packId": "core-latin",
                "path": "latin/rec.onnx"
            }]
        });
        let error = super::dry_run_managed_source_index(&manifest, &index, Some("missing-pack"))
            .unwrap_err();
        assert!(error.contains("unknown OCR model pack"));
    }
}

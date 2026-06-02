use serde_json::{json, Value};
use std::collections::HashSet;

pub(crate) const MODEL_INDEX_KIND: &str = "ysn-ocr-model-index";
pub(crate) const MANIFEST_KIND: &str = "ysn-ocr-runtime-manifest";
pub(crate) const SCHEMA_VERSION: u64 = 1;

pub(crate) fn supported_source_languages() -> Vec<&'static str> {
    vec![
        "auto", "zh-Hans", "zh-Hant", "en", "fr", "ja", "de", "es", "pt", "it", "ko", "ru", "ar",
        "th", "tr",
    ]
}

pub(crate) fn supported_target_languages() -> Vec<&'static str> {
    vec![
        "zh-Hans", "zh-Hant", "en", "fr", "ja", "de", "es", "pt", "it", "ko", "ru", "ar", "th",
        "tr",
    ]
}

pub(crate) fn supported_scripts() -> Vec<&'static str> {
    vec![
        "mixed", "cjk", "latin", "hangul", "cyrillic", "arabic", "thai",
    ]
}

pub(crate) fn model_schema() -> Value {
    json!({
        "kind": MODEL_INDEX_KIND,
        "schemaVersion": SCHEMA_VERSION,
        "sourceLanguagePolicy": {
            "selection": "automatic-only",
            "default": "auto",
            "supported": supported_source_languages()
        },
        "targetLanguagePolicy": {
            "selection": "user-selectable",
            "default": "zh-Hans",
            "supported": supported_target_languages()
        },
        "scriptRouting": {
            "strategy": "detect-script-confidence-then-recognize",
            "supportedScripts": supported_scripts(),
            "requiresDetectionModel": true,
            "requiresRecognitionFallback": true,
            "fallbackOrder": ["accurate-extension", "auto-multilingual-balanced", "compatibility-external-ocr"]
        },
        "artifactPolicy": {
            "provider": "ysn-managed",
            "transport": ["https", "localhost-dev"],
            "requiresSha256": true,
            "requiresSize": true,
            "requiresReviewedLicense": true,
            "requiresSafeRelativePath": true
        },
        "runtimeContract": {
            "engine": "onnxruntime",
            "inputImage": "rgba-screenshot-crop",
            "outputBlock": ["text", "confidence", "box_coords", "script", "language", "modelId"],
            "confidenceScale": "0..1",
            "lowConfidenceAction": "retry-with-fallback-model-or-report-not-ready"
        },
        "selfTestPolicy": {
            "requiredBeforeReady": true,
            "sampleKinds": ["latin-ui", "cjk-ui", "technical-text", "mixed-script"],
            "protectedTerms": ["PATH", "Windows", "OCR", "ONNX", "RapidOCR", "PaddleOCR-json", ".exe"]
        }
    })
}

pub(crate) fn detection_model_contract() -> Value {
    json!({
        "preprocess": { "profile": "screenshot-text-detection", "color": "rgb", "normalize": "0-1", "maxSide": 1920 },
        "decoder": { "type": "db-text-detector", "polygonOutput": true, "scoreThreshold": 0.3, "boxThreshold": 0.5 },
        "postprocess": { "mergeNearbyBoxes": true, "preserveReadingOrder": true }
    })
}

pub(crate) fn classification_model_contract() -> Value {
    json!({
        "preprocess": { "profile": "text-line-angle-classification", "color": "rgb", "height": 48 },
        "decoder": { "type": "angle-classifier", "labels": ["0", "180"] },
        "postprocess": { "rotateBeforeRecognition": true }
    })
}

pub(crate) fn recognition_model_contract(script: &str) -> Value {
    json!({
        "preprocess": { "profile": "text-line-recognition", "color": "rgb", "height": 48, "dynamicWidth": true },
        "decoder": { "type": "ctc-text-recognizer", "dictionaryScript": script, "blankToken": "ctc-blank" },
        "dictionary": dictionary_contract(script),
        "postprocess": { "normalizeWhitespace": true, "preserveTechnicalTokens": true }
    })
}

pub(crate) fn dictionary_contract(script: &str) -> Value {
    json!({
        "script": script,
        "format": "utf8-lines",
        "path": format!("dictionaries/{script}.txt"),
        "sha256": "",
        "size": 0,
        "source": { "provider": "ysn-managed", "url": "", "license": "pending-review" },
        "blankTokenId": 0,
        "reservedTokens": ["ctc-blank"],
        "requiredBeforeReady": true
    })
}

pub(crate) fn self_test_samples() -> Value {
    json!([
        {
            "id": "latin-ui-technical-path",
            "kind": "latin-ui",
            "expectedScripts": ["latin"],
            "protectedTerms": ["PATH", "Windows", ".exe"],
            "minimumConfidence": 0.82
        },
        {
            "id": "cjk-ui-short-labels",
            "kind": "cjk-ui",
            "expectedScripts": ["cjk"],
            "minimumConfidence": 0.80
        },
        {
            "id": "mixed-script-github-list",
            "kind": "mixed-script",
            "expectedScripts": ["latin", "cjk"],
            "protectedTerms": ["OCR", "ONNX", "PaddleOCR-json"],
            "minimumConfidence": 0.78
        }
    ])
}

pub(crate) fn validate_model_index_schema(index: &Value) -> Vec<Value> {
    let mut issues = validate_common_schema(index, MODEL_INDEX_KIND);
    validate_language_policy(index, &mut issues);
    validate_packs_and_models(index, &mut issues);
    issues
}

pub(crate) fn validate_manifest_schema(manifest: &Value) -> Vec<Value> {
    let mut issues = validate_common_schema(manifest, MANIFEST_KIND);
    if manifest["runtime"].as_str() != Some("ysn-ocr-runtime") {
        issues.push(json!({ "severity": "error", "code": "runtime-invalid", "message": "Manifest runtime must be ysn-ocr-runtime." }));
    }
    if manifest["defaultSourceLanguage"].as_str() != Some("auto") {
        issues.push(json!({ "severity": "error", "code": "source-language-not-auto", "message": "Source OCR language must be automatic." }));
    }
    validate_language_policy(manifest, &mut issues);
    validate_packs_and_models(manifest, &mut issues);
    issues
}

fn validate_common_schema(value: &Value, expected_kind: &str) -> Vec<Value> {
    let mut issues = Vec::new();
    if value["kind"].as_str() != Some(expected_kind) {
        issues.push(json!({ "severity": "error", "code": "kind-invalid", "message": format!("OCR schema kind must be {expected_kind}.") }));
    }
    if value["schemaVersion"].as_u64() != Some(SCHEMA_VERSION) {
        issues.push(json!({ "severity": "error", "code": "schema-version-invalid", "message": "OCR schemaVersion is unsupported." }));
    }
    if !value["modelSchema"].is_object() {
        issues.push(json!({ "severity": "error", "code": "model-schema-missing", "message": "OCR modelSchema contract is required." }));
    }
    issues
}

fn validate_language_policy(value: &Value, issues: &mut Vec<Value>) {
    let source_policy = &value["modelSchema"]["sourceLanguagePolicy"];
    if source_policy["selection"].as_str() != Some("automatic-only")
        || source_policy["default"].as_str() != Some("auto")
    {
        issues.push(json!({ "severity": "error", "code": "source-policy-invalid", "message": "Source language policy must be automatic-only with auto default." }));
    }
    let supported = source_policy["supported"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    for language in supported_source_languages() {
        if !supported.iter().any(|item| item.as_str() == Some(language)) {
            issues.push(json!({ "severity": "error", "code": "source-language-missing", "language": language, "message": "Required source language is missing from OCR schema." }));
        }
    }
    let target_policy = &value["modelSchema"]["targetLanguagePolicy"];
    if target_policy["selection"].as_str() != Some("user-selectable")
        || target_policy["default"].as_str() != Some("zh-Hans")
    {
        issues.push(json!({ "severity": "error", "code": "target-policy-invalid", "message": "Target language policy must be user-selectable with Simplified Chinese default." }));
    }
}

fn validate_packs_and_models(value: &Value, issues: &mut Vec<Value>) {
    let Some(packs) = value["packs"].as_array() else {
        issues.push(json!({ "severity": "error", "code": "packs-missing", "message": "OCR packs must be an array." }));
        return;
    };
    let Some(models) = value["models"].as_array() else {
        issues.push(json!({ "severity": "error", "code": "models-missing", "message": "OCR models must be an array." }));
        return;
    };
    let model_ids: HashSet<&str> = models
        .iter()
        .filter_map(|model| model["id"].as_str())
        .collect();
    for pack in packs {
        validate_pack(pack, &model_ids, issues);
    }
    for model in models {
        validate_model(model, issues);
    }
}

fn validate_pack(pack: &Value, model_ids: &HashSet<&str>, issues: &mut Vec<Value>) {
    let pack_id = pack["id"].as_str().unwrap_or("unknown");
    if pack_id == "unknown" {
        issues.push(json!({ "severity": "error", "code": "pack-id-missing", "message": "OCR pack id is required." }));
    }
    if pack["profile"].as_str().is_none() {
        issues.push(json!({ "severity": "error", "code": "pack-profile-missing", "packId": pack_id, "message": "OCR pack profile is required." }));
    }
    let languages = pack["languages"].as_array().cloned().unwrap_or_default();
    for language in [
        "zh-Hans", "zh-Hant", "en", "fr", "ja", "de", "es", "pt", "it", "ko", "ru", "ar", "th",
        "tr",
    ] {
        if pack["required"].as_bool().unwrap_or(false)
            && !languages.iter().any(|item| item.as_str() == Some(language))
        {
            issues.push(json!({ "severity": "error", "code": "pack-language-missing", "packId": pack_id, "language": language, "message": "Required OCR pack must cover the commercial multilingual baseline." }));
        }
    }
    let model_refs = pack["modelIds"].as_array().cloned().unwrap_or_default();
    for model_id in model_refs.iter().filter_map(|item| item.as_str()) {
        if !model_ids.contains(model_id) {
            issues.push(json!({ "severity": "error", "code": "pack-model-missing", "packId": pack_id, "modelId": model_id, "message": "OCR pack references a missing model." }));
        }
    }
}

fn validate_model(model: &Value, issues: &mut Vec<Value>) {
    let model_id = model["id"].as_str().unwrap_or("unknown");
    for field in [
        "type", "engine", "profile", "path", "version", "packId", "sha256", "size", "contract",
    ] {
        if model[field].is_null() {
            issues.push(json!({ "severity": "error", "code": "model-field-missing", "modelId": model_id, "field": field, "message": "OCR model descriptor is missing a required field." }));
        }
    }
    if model["engine"].as_str() != Some("onnxruntime") {
        issues.push(json!({ "severity": "error", "code": "model-engine-invalid", "modelId": model_id, "message": "Strategic OCR models must use the owned ONNX Runtime path." }));
    }
    if model["scripts"]
        .as_array()
        .map(|items| items.is_empty())
        .unwrap_or(true)
    {
        issues.push(json!({ "severity": "error", "code": "model-scripts-missing", "modelId": model_id, "message": "OCR model must declare script coverage." }));
    }
    if model["languages"]
        .as_array()
        .map(|items| items.is_empty())
        .unwrap_or(true)
    {
        issues.push(json!({ "severity": "error", "code": "model-languages-missing", "modelId": model_id, "message": "OCR model must declare language coverage." }));
    }
    if !model["contract"].is_object() || !model["contract"]["decoder"].is_object() {
        issues.push(json!({ "severity": "error", "code": "model-contract-invalid", "modelId": model_id, "message": "OCR model must declare preprocess, decoder, and postprocess contract." }));
    }
    if model["type"].as_str() == Some("recognition") {
        validate_recognition_dictionary_contract(model, model_id, issues);
    }
}

fn validate_recognition_dictionary_contract(
    model: &Value,
    model_id: &str,
    issues: &mut Vec<Value>,
) {
    let dictionary = &model["contract"]["dictionary"];
    if !dictionary.is_object() {
        issues.push(json!({ "severity": "error", "code": "recognition-dictionary-missing", "modelId": model_id, "message": "Recognition model must declare dictionary artifact metadata." }));
        return;
    }
    let script = dictionary["script"].as_str().unwrap_or("");
    if script.is_empty() {
        issues.push(json!({ "severity": "error", "code": "recognition-dictionary-script-missing", "modelId": model_id, "message": "Recognition dictionary must declare script coverage." }));
    } else if !model["scripts"]
        .as_array()
        .map(|scripts| scripts.iter().any(|item| item.as_str() == Some(script)))
        .unwrap_or(false)
    {
        issues.push(json!({ "severity": "error", "code": "recognition-dictionary-script-mismatch", "modelId": model_id, "script": script, "message": "Recognition dictionary script must match model script coverage." }));
    }
    let path = dictionary["path"].as_str().unwrap_or("");
    if !path.starts_with("dictionaries/") || path.contains("..") || !path.ends_with(".txt") {
        issues.push(json!({ "severity": "error", "code": "recognition-dictionary-path-invalid", "modelId": model_id, "message": "Recognition dictionary path must be a safe dictionaries/*.txt artifact." }));
    }
    if dictionary["format"].as_str() != Some("utf8-lines") {
        issues.push(json!({ "severity": "error", "code": "recognition-dictionary-format-invalid", "modelId": model_id, "message": "Recognition dictionary format must be utf8-lines." }));
    }
    if dictionary["source"]["provider"].as_str().unwrap_or("") != "ysn-managed" {
        issues.push(json!({ "severity": "error", "code": "recognition-dictionary-source-invalid", "modelId": model_id, "message": "Recognition dictionary source must be ysn-managed." }));
    }
    if dictionary["blankTokenId"].as_u64().is_none() {
        issues.push(json!({ "severity": "error", "code": "recognition-dictionary-blank-missing", "modelId": model_id, "message": "Recognition dictionary must declare blankTokenId." }));
    }
    if !dictionary["requiredBeforeReady"].as_bool().unwrap_or(false) {
        issues.push(json!({ "severity": "error", "code": "recognition-dictionary-not-required", "modelId": model_id, "message": "Recognition dictionary must be required before runtime readiness." }));
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn model_schema_requires_automatic_source_language() {
        let schema = super::model_schema();
        assert_eq!(
            schema["sourceLanguagePolicy"]["selection"].as_str(),
            Some("automatic-only")
        );
        assert!(schema["sourceLanguagePolicy"]["supported"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("tr")));
        assert_eq!(
            schema["targetLanguagePolicy"]["default"].as_str(),
            Some("zh-Hans")
        );
    }

    #[test]
    fn validator_rejects_manual_source_language_policy() {
        let mut index = json!({
            "kind": super::MODEL_INDEX_KIND,
            "schemaVersion": super::SCHEMA_VERSION,
            "modelSchema": super::model_schema(),
            "packs": [],
            "models": []
        });
        index["modelSchema"]["sourceLanguagePolicy"]["selection"] = json!("manual");
        let issues = super::validate_model_index_schema(&index);
        assert!(issues
            .iter()
            .any(|issue| issue["code"] == "source-policy-invalid"));
    }

    #[test]
    fn recognition_contract_declares_dictionary_artifact() {
        let contract = super::recognition_model_contract("latin");
        assert_eq!(contract["dictionary"]["script"].as_str(), Some("latin"));
        assert_eq!(
            contract["dictionary"]["path"].as_str(),
            Some("dictionaries/latin.txt")
        );
        assert_eq!(contract["dictionary"]["blankTokenId"].as_u64(), Some(0));
    }

    #[test]
    fn validator_rejects_recognition_dictionary_mismatch() {
        let mut index = crate::ysn_ocr_model_index::default_model_index("test");
        let models = index["models"].as_array_mut().unwrap();
        let model = models
            .iter_mut()
            .find(|model| model["id"].as_str() == Some("rec-latin"))
            .unwrap();
        model["contract"]["dictionary"]["script"] = json!("cjk");

        let issues = super::validate_model_index_schema(&index);

        assert!(issues
            .iter()
            .any(|issue| issue["code"] == "recognition-dictionary-script-mismatch"));
    }
}

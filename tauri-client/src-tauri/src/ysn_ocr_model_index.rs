use serde_json::{json, Value};

fn default_model_descriptors(model_set_version: &str) -> Value {
    json!([
        {
            "id": "det-default",
            "type": "detection",
            "engine": "onnxruntime",
            "profile": "balanced",
            "scripts": ["mixed"],
            "languages": ["auto"],
            "path": "models/det-default.onnx",
            "version": model_set_version,
            "packId": "auto-multilingual-balanced",
            "source": { "provider": "ysn-managed", "url": "", "license": "pending-review" },
            "sha256": "",
            "size": 0,
            "required": true,
            "status": "not-installed",
            "contract": crate::ysn_ocr_model_schema::detection_model_contract()
        },
        {
            "id": "cls-default",
            "type": "classification",
            "engine": "onnxruntime",
            "profile": "balanced",
            "scripts": ["mixed"],
            "languages": ["auto"],
            "path": "models/cls-default.onnx",
            "version": model_set_version,
            "packId": "auto-multilingual-balanced",
            "source": { "provider": "ysn-managed", "url": "", "license": "pending-review" },
            "sha256": "",
            "size": 0,
            "required": true,
            "status": "not-installed",
            "contract": crate::ysn_ocr_model_schema::classification_model_contract()
        },
        {
            "id": "rec-cjk",
            "type": "recognition",
            "engine": "onnxruntime",
            "profile": "balanced",
            "scripts": ["cjk"],
            "languages": ["zh-Hans", "zh-Hant", "ja"],
            "path": "models/rec-cjk.onnx",
            "version": model_set_version,
            "packId": "auto-multilingual-balanced",
            "source": { "provider": "ysn-managed", "url": "", "license": "pending-review" },
            "sha256": "",
            "size": 0,
            "required": true,
            "status": "not-installed",
            "contract": crate::ysn_ocr_model_schema::recognition_model_contract("cjk")
        },
        {
            "id": "rec-latin",
            "type": "recognition",
            "engine": "onnxruntime",
            "profile": "balanced",
            "scripts": ["latin"],
            "languages": ["en", "fr", "de", "es", "pt", "it", "tr"],
            "path": "models/rec-latin.onnx",
            "version": model_set_version,
            "packId": "auto-multilingual-balanced",
            "source": { "provider": "ysn-managed", "url": "", "license": "pending-review" },
            "sha256": "",
            "size": 0,
            "required": true,
            "status": "not-installed",
            "contract": crate::ysn_ocr_model_schema::recognition_model_contract("latin")
        },
        {
            "id": "rec-korean",
            "type": "recognition",
            "engine": "onnxruntime",
            "profile": "balanced",
            "scripts": ["hangul"],
            "languages": ["ko"],
            "path": "models/rec-korean.onnx",
            "version": model_set_version,
            "packId": "auto-multilingual-balanced",
            "source": { "provider": "ysn-managed", "url": "", "license": "pending-review" },
            "sha256": "",
            "size": 0,
            "required": true,
            "status": "not-installed",
            "contract": crate::ysn_ocr_model_schema::recognition_model_contract("hangul")
        },
        {
            "id": "rec-cyrillic",
            "type": "recognition",
            "engine": "onnxruntime",
            "profile": "balanced",
            "scripts": ["cyrillic"],
            "languages": ["ru"],
            "path": "models/rec-cyrillic.onnx",
            "version": model_set_version,
            "packId": "auto-multilingual-balanced",
            "source": { "provider": "ysn-managed", "url": "", "license": "pending-review" },
            "sha256": "",
            "size": 0,
            "required": true,
            "status": "not-installed",
            "contract": crate::ysn_ocr_model_schema::recognition_model_contract("cyrillic")
        },
        {
            "id": "rec-arabic",
            "type": "recognition",
            "engine": "onnxruntime",
            "profile": "balanced",
            "scripts": ["arabic"],
            "languages": ["ar"],
            "path": "models/rec-arabic.onnx",
            "version": model_set_version,
            "packId": "auto-multilingual-balanced",
            "source": { "provider": "ysn-managed", "url": "", "license": "pending-review" },
            "sha256": "",
            "size": 0,
            "required": true,
            "status": "not-installed",
            "contract": crate::ysn_ocr_model_schema::recognition_model_contract("arabic")
        },
        {
            "id": "rec-thai",
            "type": "recognition",
            "engine": "onnxruntime",
            "profile": "balanced",
            "scripts": ["thai"],
            "languages": ["th"],
            "path": "models/rec-thai.onnx",
            "version": model_set_version,
            "packId": "auto-multilingual-balanced",
            "source": { "provider": "ysn-managed", "url": "", "license": "pending-review" },
            "sha256": "",
            "size": 0,
            "required": true,
            "status": "not-installed",
            "contract": crate::ysn_ocr_model_schema::recognition_model_contract("thai")
        }
    ])
}

pub fn default_model_index(model_set_version: &str) -> Value {
    json!({
        "kind": crate::ysn_ocr_model_schema::MODEL_INDEX_KIND,
        "schemaVersion": crate::ysn_ocr_model_schema::SCHEMA_VERSION,
        "updatedAt": "2026-06-02T00:00:00+08:00",
        "minimumAppVersion": "1.1.0",
        "modelSchema": crate::ysn_ocr_model_schema::model_schema(),
        "selfTestSamples": crate::ysn_ocr_model_schema::self_test_samples(),
        "sourcePolicy": crate::ysn_ocr_model_sources::trusted_source_policy(),
        "packs": [
            {
                "id": "auto-multilingual-balanced",
                "name": {
                    "zh-CN": "Auto Multilingual OCR Pack",
                    "en-US": "Auto Multilingual OCR Pack"
                },
                "profile": "balanced",
                "required": true,
                "languages": ["zh-Hans", "zh-Hant", "en", "fr", "ja", "de", "es", "pt", "it", "ko", "ru", "ar", "th", "tr"],
                "scripts": ["cjk", "latin", "hangul", "cyrillic", "arabic", "thai"],
                "modelIds": ["det-default", "cls-default", "rec-cjk", "rec-latin", "rec-korean", "rec-cyrillic", "rec-arabic", "rec-thai"],
                "status": "not-installed"
            },
            {
                "id": "accurate-extension",
                "name": {
                    "zh-CN": "Accurate OCR Extension Pack",
                    "en-US": "Accurate OCR Extension Pack"
                },
                "profile": "accurate",
                "required": false,
                "languages": ["zh-Hans", "zh-Hant", "en", "fr", "ja", "de", "es", "pt", "it", "ko", "ru", "ar", "th", "tr"],
                "scripts": ["cjk", "latin", "hangul", "cyrillic", "arabic", "thai"],
                "modelIds": [],
                "status": "not-installed"
            }
        ],
        "models": default_model_descriptors(model_set_version)
    })
}

pub fn default_manifest(runtime_version: &str, model_set_version: &str) -> Value {
    let index = default_model_index(model_set_version);
    json!({
        "kind": crate::ysn_ocr_model_schema::MANIFEST_KIND,
        "schemaVersion": crate::ysn_ocr_model_schema::SCHEMA_VERSION,
        "runtime": "ysn-ocr-runtime",
        "runtimeVersion": runtime_version,
        "modelSetVersion": model_set_version,
        "defaultSourceLanguage": "auto",
        "defaultProfile": "balanced",
        "installedAt": null,
        "lastSelfTestAt": null,
        "modelSchema": index["modelSchema"].clone(),
        "selfTestSamples": index["selfTestSamples"].clone(),
        "sourcePolicy": index["sourcePolicy"].clone(),
        "packs": index["packs"].clone(),
        "models": index["models"].clone()
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn default_model_index_matches_commercial_schema() {
        let index = super::default_model_index("2026.06.ocr.v1");
        let issues = crate::ysn_ocr_model_schema::validate_model_index_schema(&index);
        assert!(issues.is_empty(), "default model index issues: {issues:?}");
        assert_eq!(
            index["kind"].as_str(),
            Some(crate::ysn_ocr_model_schema::MODEL_INDEX_KIND)
        );
        assert_eq!(
            index["modelSchema"]["sourceLanguagePolicy"]["selection"].as_str(),
            Some("automatic-only")
        );
        assert_eq!(
            index["modelSchema"]["targetLanguagePolicy"]["default"].as_str(),
            Some("zh-Hans")
        );
        assert!(index["selfTestSamples"]
            .as_array()
            .unwrap()
            .iter()
            .any(|sample| sample["protectedTerms"]
                .as_array()
                .unwrap()
                .iter()
                .any(|term| term.as_str() == Some("PATH"))));
    }

    #[test]
    fn default_manifest_keeps_source_language_automatic() {
        let manifest = super::default_manifest("1.1.0", "2026.06.ocr.v1");
        let issues = crate::ysn_ocr_model_schema::validate_manifest_schema(&manifest);
        assert!(issues.is_empty(), "default manifest issues: {issues:?}");
        assert_eq!(
            manifest["kind"].as_str(),
            Some(crate::ysn_ocr_model_schema::MANIFEST_KIND)
        );
        assert_eq!(manifest["defaultSourceLanguage"].as_str(), Some("auto"));
        assert!(manifest["packs"][0]["languages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|language| language.as_str() == Some("tr")));
        assert!(manifest["packs"][0]["languages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|language| language.as_str() == Some("zh-Hant")));
    }
}

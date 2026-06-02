use serde_json::{json, Value};

fn script_for_char(ch: char) -> Option<&'static str> {
    let code = ch as u32;
    match code {
        0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0x3040..=0x30FF => Some("cjk"),
        0xAC00..=0xD7AF | 0x1100..=0x11FF | 0x3130..=0x318F => Some("hangul"),
        0x0400..=0x052F => Some("cyrillic"),
        0x0600..=0x06FF | 0x0750..=0x077F | 0x08A0..=0x08FF => Some("arabic"),
        0x0E00..=0x0E7F => Some("thai"),
        0x0041..=0x005A | 0x0061..=0x007A | 0x00C0..=0x024F | 0x1E00..=0x1EFF => Some("latin"),
        _ => None,
    }
}

fn script_counts(text: &str) -> std::collections::BTreeMap<String, usize> {
    let mut counts = std::collections::BTreeMap::new();
    for ch in text.chars() {
        if let Some(script) = script_for_char(ch) {
            *counts.entry(script.to_string()).or_insert(0) += 1;
        }
    }
    counts
}

fn dominant_script(counts: &std::collections::BTreeMap<String, usize>) -> String {
    counts
        .iter()
        .max_by_key(|(_, count)| *count)
        .map(|(script, _)| script.clone())
        .unwrap_or_else(|| "unknown".to_string())
}

fn recognizers_for_script<'a>(manifest: &'a Value, script: &str) -> Vec<&'a Value> {
    manifest["models"]
        .as_array()
        .map(|models| {
            models
                .iter()
                .filter(|model| model["type"].as_str() == Some("recognition"))
                .filter(|model| model["status"].as_str() == Some("installed"))
                .filter(|model| {
                    model["scripts"]
                        .as_array()
                        .map(|scripts| {
                            scripts.iter().any(|value| {
                                value.as_str() == Some(script) || value.as_str() == Some("mixed")
                            })
                        })
                        .unwrap_or(false)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn fallback_recognizers<'a>(manifest: &'a Value) -> Vec<&'a Value> {
    manifest["models"]
        .as_array()
        .map(|models| {
            models
                .iter()
                .filter(|model| model["type"].as_str() == Some("recognition"))
                .filter(|model| model["status"].as_str() == Some("installed"))
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn build_route_plan(manifest: &Value, texts: &[String]) -> Value {
    let mut line_routes = Vec::new();
    let mut missing_scripts = std::collections::BTreeSet::new();

    for (index, text) in texts.iter().enumerate() {
        let counts = script_counts(text);
        let script = dominant_script(&counts);
        let mut candidates = if script == "unknown" {
            Vec::new()
        } else {
            recognizers_for_script(manifest, &script)
        };
        let route_reason = if candidates.is_empty() {
            missing_scripts.insert(script.clone());
            candidates = fallback_recognizers(manifest);
            if candidates.is_empty() {
                "no-installed-recognizer"
            } else {
                "fallback-installed-recognizer"
            }
        } else {
            "script-match"
        };
        let candidate_models: Vec<Value> = candidates
            .iter()
            .map(|model| {
                json!({
                    "modelId": model["id"].clone(),
                    "scripts": model["scripts"].clone(),
                    "languages": model["languages"].clone(),
                    "profile": model["profile"].clone(),
                    "sourceProvider": model["source"]["provider"].clone()
                })
            })
            .collect();

        line_routes.push(json!({
            "index": index,
            "textSample": text.chars().take(120).collect::<String>(),
            "dominantScript": script,
            "scriptCounts": counts,
            "routeReason": route_reason,
            "candidateModels": candidate_models,
            "needsFallback": route_reason != "script-match"
        }));
    }

    json!({
        "sourceLanguage": "auto",
        "lineCount": texts.len(),
        "routes": line_routes,
        "missingScripts": missing_scripts.into_iter().collect::<Vec<String>>(),
        "policy": "script-first; confidence retry and VLM fallback are planned but not wired yet"
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    fn manifest() -> serde_json::Value {
        json!({
            "models": [
                {
                    "id": "latin-rec",
                    "type": "recognition",
                    "status": "installed",
                    "scripts": ["latin"],
                    "languages": ["en", "fr", "de"],
                    "profile": "recognition-latin",
                    "source": { "provider": "ysn-managed" }
                },
                {
                    "id": "cjk-rec",
                    "type": "recognition",
                    "status": "installed",
                    "scripts": ["cjk"],
                    "languages": ["zh-Hans", "ja"],
                    "profile": "recognition-cjk",
                    "source": { "provider": "ysn-managed" }
                }
            ]
        })
    }

    #[test]
    fn test_route_plan_matches_latin_and_cjk() {
        let plan = super::build_route_plan(
            &manifest(),
            &[
                "Add PATH".to_string(),
                "\u{5b89}\u{88c5}\u{4e2d}\u{6587}\u{6a21}\u{578b}".to_string(),
            ],
        );
        assert_eq!(plan["sourceLanguage"].as_str(), Some("auto"));
        assert_eq!(plan["lineCount"].as_u64(), Some(2));
        assert_eq!(plan["routes"][0]["dominantScript"].as_str(), Some("latin"));
        assert_eq!(
            plan["routes"][0]["routeReason"].as_str(),
            Some("script-match")
        );
        assert_eq!(
            plan["routes"][0]["candidateModels"][0]["modelId"].as_str(),
            Some("latin-rec")
        );
        assert_eq!(plan["routes"][1]["dominantScript"].as_str(), Some("cjk"));
        assert_eq!(
            plan["routes"][1]["candidateModels"][0]["modelId"].as_str(),
            Some("cjk-rec")
        );
    }

    #[test]
    fn test_route_plan_reports_missing_script_and_uses_fallback() {
        let plan = super::build_route_plan(
            &manifest(),
            &["\u{0645}\u{0631}\u{062d}\u{0628}\u{0627}".to_string()],
        );
        assert_eq!(plan["routes"][0]["dominantScript"].as_str(), Some("arabic"));
        assert_eq!(
            plan["routes"][0]["routeReason"].as_str(),
            Some("fallback-installed-recognizer")
        );
        assert_eq!(plan["routes"][0]["needsFallback"].as_bool(), Some(true));
        assert!(plan["missingScripts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("arabic")));
    }
}

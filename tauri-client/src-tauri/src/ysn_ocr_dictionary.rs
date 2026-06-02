use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LoadedOcrDictionary {
    pub script: String,
    pub relative_path: String,
    pub absolute_path: String,
    pub sha256: String,
    pub size: u64,
    pub blank_token_id: usize,
    pub token_count: usize,
    #[serde(skip_serializing)]
    pub tokens: Vec<String>,
}

pub fn load_dictionary_from_utf8_lines(
    content: &str,
    blank_token_id: usize,
) -> Result<Vec<String>, String> {
    let tokens: Vec<String> = content.lines().map(|line| line.to_string()).collect();
    validate_dictionary_tokens(&tokens, blank_token_id)?;
    Ok(tokens)
}

pub fn load_dictionary_from_contract(
    active_root: &Path,
    contract: &Value,
) -> Result<LoadedOcrDictionary, String> {
    let dictionary = &contract["dictionary"];
    let script = dictionary["script"]
        .as_str()
        .ok_or_else(|| "OCR dictionary script is required.".to_string())?;
    let relative_path = dictionary["path"]
        .as_str()
        .ok_or_else(|| "OCR dictionary path is required.".to_string())?;
    let blank_token_id = dictionary["blankTokenId"]
        .as_u64()
        .ok_or_else(|| "OCR dictionary blankTokenId is required.".to_string())?
        as usize;
    if !is_safe_dictionary_path(relative_path) {
        return Err(format!("unsafe OCR dictionary path: {relative_path}"));
    }
    let path = active_root.join(relative_path);
    if !path.is_file() {
        return Err(format!(
            "OCR dictionary file is missing: {}",
            path.to_string_lossy()
        ));
    }
    let metadata = fs::metadata(&path)
        .map_err(|error| format!("failed to inspect OCR dictionary file: {error}"))?;
    let expected_size = dictionary["size"].as_u64().unwrap_or(0);
    if expected_size > 0 && metadata.len() != expected_size {
        return Err(format!(
            "OCR dictionary size mismatch: expected {expected_size}, got {}.",
            metadata.len()
        ));
    }
    let expected_sha256 = dictionary["sha256"].as_str().unwrap_or("");
    let actual_sha256 = crate::ysn_ocr_model_downloader::sha256_file(&path)?;
    if crate::ysn_ocr_model_downloader::is_sha256_hex(expected_sha256)
        && !actual_sha256.eq_ignore_ascii_case(expected_sha256)
    {
        return Err("OCR dictionary SHA256 mismatch.".to_string());
    }
    let content = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read OCR dictionary file as UTF-8: {error}"))?;
    let tokens = load_dictionary_from_utf8_lines(&content, blank_token_id)?;
    Ok(LoadedOcrDictionary {
        script: script.to_string(),
        relative_path: relative_path.to_string(),
        absolute_path: path.to_string_lossy().to_string(),
        sha256: actual_sha256,
        size: metadata.len(),
        blank_token_id,
        token_count: tokens.len(),
        tokens,
    })
}

pub fn is_safe_dictionary_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    crate::ysn_ocr_model_downloader::is_safe_relative_model_path(&normalized)
        && normalized.starts_with("dictionaries/")
        && normalized.ends_with(".txt")
}

fn validate_dictionary_tokens(tokens: &[String], blank_token_id: usize) -> Result<(), String> {
    if tokens.len() < 2 {
        return Err("OCR dictionary must contain at least blank and one text token.".to_string());
    }
    if blank_token_id >= tokens.len() {
        return Err(format!(
            "OCR dictionary blank token id {blank_token_id} is outside token count {}.",
            tokens.len()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use sha2::{Digest, Sha256};
    use std::fs;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "ysn-ocr-dictionary-test-{name}-{}",
            chrono::Local::now()
                .timestamp_nanos_opt()
                .unwrap_or_default()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn sha256_text(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    #[test]
    fn load_dictionary_from_utf8_lines_keeps_blank_token() {
        let tokens = super::load_dictionary_from_utf8_lines("\nA\nB\n", 0).unwrap();

        assert_eq!(tokens, vec!["", "A", "B"]);
    }

    #[test]
    fn load_dictionary_from_utf8_lines_rejects_blank_outside_tokens() {
        let error = super::load_dictionary_from_utf8_lines("\nA\n", 3).unwrap_err();

        assert!(error.contains("outside token count"));
    }

    #[test]
    fn is_safe_dictionary_path_rejects_escape_paths() {
        assert!(super::is_safe_dictionary_path("dictionaries/latin.txt"));
        assert!(!super::is_safe_dictionary_path("../latin.txt"));
        assert!(!super::is_safe_dictionary_path("models/latin.txt"));
        assert!(!super::is_safe_dictionary_path("dictionaries/latin.bin"));
    }

    #[test]
    fn load_dictionary_from_contract_verifies_size_and_sha() {
        let root = temp_dir("ok");
        let dictionary_dir = root.join("dictionaries");
        fs::create_dir_all(&dictionary_dir).unwrap();
        let content = "\nA\nB\n";
        fs::write(dictionary_dir.join("latin.txt"), content).unwrap();
        let contract = json!({
            "dictionary": {
                "script": "latin",
                "path": "dictionaries/latin.txt",
                "sha256": sha256_text(content),
                "size": content.len() as u64,
                "blankTokenId": 0
            }
        });

        let dictionary = super::load_dictionary_from_contract(&root, &contract).unwrap();

        assert_eq!(dictionary.script, "latin");
        assert_eq!(dictionary.token_count, 3);
        assert_eq!(dictionary.tokens[1], "A");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn load_dictionary_from_contract_rejects_sha_mismatch() {
        let root = temp_dir("sha");
        let dictionary_dir = root.join("dictionaries");
        fs::create_dir_all(&dictionary_dir).unwrap();
        fs::write(dictionary_dir.join("latin.txt"), "\nA\n").unwrap();
        let contract = json!({
            "dictionary": {
                "script": "latin",
                "path": "dictionaries/latin.txt",
                "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "size": 3,
                "blankTokenId": 0
            }
        });

        let error = super::load_dictionary_from_contract(&root, &contract).unwrap_err();

        assert!(error.contains("SHA256 mismatch"));
        let _ = fs::remove_dir_all(root);
    }
}

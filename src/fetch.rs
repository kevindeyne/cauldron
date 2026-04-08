use std::collections::HashMap;
use crate::model::{Cache, CachedEntry, GitTreeResponse, RemoteEntry, ToolConfig};
use crate::util::now_secs;

const REPO_BASE: &str = "https://raw.githubusercontent.com/kevindeyne/cauldron-recipes/main";
const REPO_API: &str = "https://api.github.com/repos/kevindeyne/cauldron-recipes/git/trees/main?recursive=1";

pub fn fetch_text(url: &str) -> Result<String, String> {
    let agent = ureq::Agent::new_with_defaults();
    let response = agent
        .get(url)
        .header("User-Agent", "cauldron-sdk-cli/1.0")
        .call()
        .map_err(|e| format!("Request failed: {}", e))?;

    response
        .into_body()
        .read_to_string()
        .map_err(|e| format!("Failed to read response body: {}", e))
}

pub fn fetch_tool_config(tool: &str) -> Result<ToolConfig, String> {
    let url = format!("{}/{}/_setup.json", REPO_BASE, tool);
    let body = fetch_text(&url)?;
    serde_json::from_str(&body).map_err(|e| format!("Invalid tool config for '{}': {}", tool, e))
}

pub fn fetch_remote() -> Result<Cache, String> {
    let body = fetch_text(REPO_API)?;
    let tree: GitTreeResponse = serde_json::from_str(&body).map_err(|e| e.to_string())?;

    let json_paths: Vec<String> = tree
        .tree
        .into_iter()
        .filter(|item| item.kind == "blob" && item.path.ends_with(".json"))
        .map(|item| item.path)
        .collect();

    let mut data: HashMap<String, HashMap<String, Vec<CachedEntry>>> = HashMap::new();

    for path in json_paths {
        let (category, vendor) = parse_path(&path);

        let raw_url = format!("{}/{}", REPO_BASE, path);
        let raw = match fetch_text(&raw_url) {
            Ok(t) => t,
            Err(_) => continue,
        };

        let entries: Vec<RemoteEntry> = match serde_json::from_str(&raw) {
            Ok(e) => e,
            Err(_) => continue,
        };

        let cached: Vec<CachedEntry> = entries
            .into_iter()
            .map(|e| CachedEntry { version: e.version, url: e.url, checksums: e.checksums })
            .collect();

        data.entry(category).or_default().insert(vendor, cached);
    }

    Ok(Cache { fetched_at: now_secs(), data })
}

/// Derive (category, vendor) from a repo-relative path like "java/corretto.json" or "maven.json".
pub fn parse_path(path: &str) -> (String, String) {
    let parts: Vec<&str> = path.splitn(3, '/').collect();
    match parts.as_slice() {
        [file] => {
            let name = file.trim_end_matches(".json").to_string();
            (name.clone(), name)
        }
        [cat, vendor_file] => (
            cat.to_string(),
            vendor_file.trim_end_matches(".json").to_string(),
        ),
        [cat, subdir, vendor_file] => (
            cat.to_string(),
            format!("{}/{}", subdir, vendor_file.trim_end_matches(".json")),
        ),
        _ => (path.to_string(), path.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_top_level_file() {
        let (cat, vendor) = parse_path("maven.json");
        assert_eq!(cat, "maven");
        assert_eq!(vendor, "maven");
    }

    #[test]
    fn parse_category_vendor() {
        let (cat, vendor) = parse_path("java/corretto.json");
        assert_eq!(cat, "java");
        assert_eq!(vendor, "corretto");
    }

    #[test]
    fn parse_nested_vendor() {
        let (cat, vendor) = parse_path("java/corretto/lts.json");
        assert_eq!(cat, "java");
        assert_eq!(vendor, "corretto/lts");
    }

    #[test]
    fn parse_strips_json_extension() {
        let (_, vendor) = parse_path("java/adoptium.json");
        assert!(!vendor.contains(".json"));
    }
}
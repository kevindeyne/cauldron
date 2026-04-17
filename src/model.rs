use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct GitTreeResponse {
    pub tree: Vec<GitTreeItem>,
}

#[derive(Deserialize)]
pub struct GitTreeItem {
    pub path: String,
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Deserialize)]
pub struct RemoteEntry {
    pub version: String,
    pub url: String,
    pub checksums: Option<Checksums>
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CachedEntry {
    pub version: String,
    pub url: String,
    pub checksums: Option<Checksums>
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Checksums {
    pub md5: Option<String>,
    #[serde(rename = "SHA-256")]
    pub sha256: Option<String>,
    #[serde(rename = "SHA-512")]
    pub sha512: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Cache {
    pub fetched_at: u64,
    /// category -> vendor -> entries
    pub data: HashMap<String, HashMap<String, Vec<CachedEntry>>>,
}

#[derive(Deserialize)]
pub struct ToolConfig {
    pub home_var: String,   // e.g. "JAVA_HOME"
    pub bin_subdir: String, // e.g. "bin"
    pub default_vendor: Option<String>, // default vendor for this category
}
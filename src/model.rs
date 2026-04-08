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
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CachedEntry {
    pub version: String,
    pub url: String,
}

#[derive(Serialize, Deserialize)]
pub struct Cache {
    pub fetched_at: u64,
    /// category -> vendor -> entries
    pub data: HashMap<String, HashMap<String, Vec<CachedEntry>>>,
}
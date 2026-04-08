use std::fs;
use crate::fetch::fetch_remote;
use crate::model::Cache;
use crate::util::{cache_path, now_secs};

pub fn get() -> Cache {
    let local = load();

    if let Some(ref c) = local {
        if is_fresh(c) {
            return local.unwrap();
        }
    }

    match fetch_remote() {
        Ok(fresh) => {
            save(&fresh);
            fresh
        }
        Err(e) => {
            if let Some(c) = local {
                eprintln!("Warning: could not fetch remote data ({}), using cached copy.", e);
                c
            } else {
                eprintln!("Error: no local cache and remote fetch failed: {}", e);
                std::process::exit(1);
            }
        }
    }
}

fn load() -> Option<Cache> {
    let text = fs::read_to_string(cache_path()).ok()?;
    serde_json::from_str(&text).ok()
}

fn save(cache: &Cache) {
    let path = cache_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(cache) {
        let _ = fs::write(&path, json);
    }
}

const DAILY: u64 = 86_400;

fn is_fresh(cache: &Cache) -> bool {
    now_secs().saturating_sub(cache.fetched_at) < DAILY
}
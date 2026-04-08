use std::{env, path::PathBuf, time::{Duration, SystemTime, UNIX_EPOCH}};

pub fn cauldron_dir() -> PathBuf {
    let home = env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".cauldron")
}

pub fn cache_path() -> PathBuf {
    cauldron_dir().join("cache.json")
}

pub fn candidates_dir(tool: &str, vendor: &str, version: &str) -> PathBuf {
    cauldron_dir().join("candidates").join(tool).join(vendor).join(version)
}

pub fn junction_path(tool: &str) -> PathBuf {
    cauldron_dir().join("current").join(tool)
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}

/// Compare version strings numerically where possible.
pub fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let a_num: Option<u64> = a.parse().ok();
    let b_num: Option<u64> = b.parse().ok();
    match (a_num, b_num) {
        (Some(x), Some(y)) => x.cmp(&y),
        _ => a.cmp(b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    #[test]
    fn numeric_versions_compared_numerically() {
        assert_eq!(version_cmp("21", "25"), Ordering::Less);
        assert_eq!(version_cmp("25", "21"), Ordering::Greater);
        assert_eq!(version_cmp("21", "21"), Ordering::Equal);
    }

    #[test]
    fn large_version_numbers() {
        assert_eq!(version_cmp("9", "11"), Ordering::Less);
        assert_eq!(version_cmp("11", "9"), Ordering::Greater);
    }

    #[test]
    fn non_numeric_versions_compared_lexically() {
        assert_eq!(version_cmp("lts", "stable"), Ordering::Less);
        assert_eq!(version_cmp("alpha", "beta"), Ordering::Less);
    }

    #[test]
    fn cache_path_ends_correctly() {
        let path = cache_path();
        assert!(path.ends_with(".cauldron/cache.json") || path.ends_with(r".cauldron\cache.json"));
    }
}
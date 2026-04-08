use std::{fs, path::Path};

/// Remove existing junction/dir at `link` and create a new junction pointing to `target`.
pub fn set_junction(link: &Path, target: &Path) -> Result<(), String> {
    if link.exists() || link.symlink_metadata().is_ok() {
        // junction! crate or fs::remove_dir works for junctions
        fs::remove_dir(link)
            .map_err(|e| format!("Failed to remove existing junction {:?}: {}", link, e))?;
    }

    if let Some(parent) = link.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create junction parent dir: {}", e))?;
    }

    junction::create(target, link)
        .map_err(|e| format!("Failed to create junction {:?} -> {:?}: {}", link, target, e))?;

    Ok(())
}
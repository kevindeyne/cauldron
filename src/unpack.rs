use std::{
    fs::{self, File},
    io,
    path::{Path, PathBuf},
};
use zip::ZipArchive;

/// Unpack a zip to `dest`, stripping the single top-level folder if one exists.
pub fn unpack_zip(zip_path: &str, dest: &Path) -> Result<(), String> {
    let file = File::open(zip_path).map_err(|e| format!("Cannot open zip: {}", e))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("Invalid zip: {}", e))?;

    let prefix = common_prefix(&mut archive)?;

    fs::create_dir_all(dest).map_err(|e| format!("Cannot create dest dir: {}", e))?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let raw = entry.mangled_name();

        // Strip the common top-level prefix
        let relative = match prefix {
            Some(ref p) => raw.strip_prefix(p).unwrap_or(&raw).to_path_buf(),
            None => raw,
        };

        // Skip the root dir entry itself
        if relative.as_os_str().is_empty() {
            continue;
        }

        let out_path = dest.join(&relative);

        if entry.is_dir() {
            fs::create_dir_all(&out_path)
                .map_err(|e| format!("Cannot create dir {:?}: {}", out_path, e))?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Cannot create dir {:?}: {}", parent, e))?;
            }
            let mut out = File::create(&out_path)
                .map_err(|e| format!("Cannot create file {:?}: {}", out_path, e))?;
            io::copy(&mut entry, &mut out)
                .map_err(|e| format!("Cannot write file {:?}: {}", out_path, e))?;
        }
    }

    Ok(())
}

/// Detect a single common top-level directory in the zip, if present.
fn common_prefix(archive: &mut ZipArchive<File>) -> Result<Option<PathBuf>, String> {
    let mut prefix: Option<PathBuf> = None;

    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let path = entry.mangled_name();
        let root = match path.components().next() {
            Some(c) => PathBuf::from(c.as_os_str()),
            None => continue,
        };

        match prefix {
            None => prefix = Some(root),
            Some(ref p) if p != &root => return Ok(None), // multiple roots → no stripping
            _ => {}
        }
    }

    Ok(prefix)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;
    use zip::write::{FileOptions, ZipWriter};

    fn make_zip(entries: &[&str]) -> Vec<u8> {
        let mut buf = Vec::new();
        let cursor = std::io::Cursor::new(&mut buf);
        let mut zip = ZipWriter::new(cursor);
        let opts = FileOptions::<()>::default();
        for &name in entries {
            if name.ends_with('/') {
                zip.add_directory(name, opts).unwrap();
            } else {
                zip.start_file(name, opts).unwrap();
                zip.write_all(b"data").unwrap();
            }
        }
        zip.finish().unwrap();
        buf
    }

    #[test]
    fn flattens_single_top_level_dir() {
        let zip_bytes = make_zip(&["corretto-21/", "corretto-21/bin/java.exe"]);
        let dir = tempdir().unwrap();
        let zip_path = dir.path().join("test.zip");
        fs::write(&zip_path, &zip_bytes).unwrap();

        let dest = dir.path().join("out");
        unpack_zip(&zip_path.to_str().unwrap(), &dest).unwrap();

        assert!(dest.join("bin/java.exe").exists());
        assert!(!dest.join("corretto-21").exists());
    }

    #[test]
    fn keeps_structure_with_multiple_roots() {
        let zip_bytes = make_zip(&["bin/java.exe", "lib/rt.jar"]);
        let dir = tempdir().unwrap();
        let zip_path = dir.path().join("test.zip");
        fs::write(&zip_path, &zip_bytes).unwrap();

        let dest = dir.path().join("out");
        unpack_zip(&zip_path.to_str().unwrap(), &dest).unwrap();

        assert!(dest.join("bin/java.exe").exists());
        assert!(dest.join("lib/rt.jar").exists());
    }
}
use crate::{cache, env_update, fetch, junction_setup, unpack, util};
use anyhow::{bail, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::blocking::Client;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const MAX_CACHED_DOWNLOADS: usize = 2;

pub fn run(category: &str, vendor: &str, version: &str) {
    let cache = cache::get();

    let entry = cache
        .data
        .get(category)
        .and_then(|vendors| vendors.get(vendor))
        .and_then(|entries| entries.iter().find(|e| e.version == version))
        .unwrap_or_else(|| {
            eprintln!("'{}' '{}' version '{}' not found. Run 'sdk list {}' to see available options.", vendor, version, category, category);
            std::process::exit(1);
        });

    let checksum = entry
        .checksums
        .as_ref()
        .and_then(|c| c.sha256.as_ref())
        .unwrap_or_else(|| {
            eprintln!("No SHA-256 checksum available for this entry.");
            std::process::exit(1);
        });

    let candidate_dir = util::candidates_dir(category, vendor, version);
    let junction_path = util::junction_path(category);

    if let Err(e) = download(&entry.url, checksum, &candidate_dir, &junction_path, category) {
        eprintln!("Install failed: {:#}", e);
        std::process::exit(1);
    }

    println!("Done! {} {} {} is now active.", category, vendor, version);
}

fn download(url: &str, checksum: &str, candidate_dir: &Path, junction_path: &Path, category: &str) -> Result<()> {
    let downloads_dir = util::cauldron_dir().join("downloads");
    fs::create_dir_all(&downloads_dir).context("Cannot create downloads dir")?;

    let filename = url.split('/').last().context("URL has no filename segment")?;
    let zip_path = downloads_dir.join(filename);

    if zip_path.exists() {
        println!("Already downloaded: {}", zip_path.display());
        let now = filetime::FileTime::now();
        filetime::set_file_mtime(&zip_path, now).ok();
    } else {
        fetch_to_disk(url, &zip_path)?;
    }
    evict_old_downloads(&downloads_dir);
    verify_checksum(&zip_path, checksum)?;
    unpack_and_link(&zip_path, candidate_dir, junction_path, category)
}

fn evict_old_downloads(downloads_dir: &Path) {
    let mut zips: Vec<(std::time::SystemTime, PathBuf)> = fs::read_dir(downloads_dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().map_or(false, |x| x == "zip"))
        .filter_map(|e| {
            let modified = e.metadata().ok()?.modified().ok()?;
            Some((modified, e.path()))
        })
        .collect();

    // Newest first
    zips.sort_by(|a, b| b.0.cmp(&a.0));

    for (_, path) in zips.into_iter().skip(MAX_CACHED_DOWNLOADS) {
        println!("Removing old download: {}", path.display());
        fs::remove_file(&path).ok();
    }
}

fn fetch_to_disk(url: &str, zip_path: &Path) -> Result<()> {
    println!("Downloading: {}", zip_path.display());

    let client = Client::new();
    let mut resp = client.get(url).send().context("HTTP request failed")?;

    let total = resp.content_length().context("No content-length header")?;
    let pb = ProgressBar::new(total);
    pb.set_style(ProgressStyle::with_template(
        "{bar:40.cyan/blue} {bytes}/{total_bytes} ({eta})",
    )?);

    let mut file = File::create(zip_path).context("Cannot create zip file")?;
    let mut buf = [0u8; 8192];
    let mut downloaded = 0u64;

    loop {
        let n = resp.read(&mut buf).context("Error reading response")?;
        if n == 0 { break; }
        file.write_all(&buf[..n]).context("Error writing zip")?;
        downloaded += n as u64;
        pb.set_position(downloaded);
    }

    pb.finish_with_message("Download complete");
    Ok(())
}

fn verify_checksum(zip_path: &Path, expected: &str) -> Result<()> {
    println!("Verifying checksum...");

    let mut file = File::open(zip_path).context("Cannot open zip for checksum")?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];

    loop {
        let n = file.read(&mut buf).context("Error reading zip for checksum")?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }

    let actual = hasher.finalize();
    let expected_bytes = hex::decode(expected.trim()).context("Invalid checksum hex")?;

    if actual[..] != expected_bytes[..] {
        bail!("Checksum mismatch — file may be corrupt or tampered with");
    }

    println!("Checksum OK");
    Ok(())
}

fn unpack_and_link(zip_path: &Path, candidate_dir: &Path, junction_path: &Path, category: &str) -> Result<()> {
    println!("Unpacking to {}...", candidate_dir.display());
    unpack::unpack_zip(zip_path.to_str().unwrap(), candidate_dir).expect("Unpack failed");

    println!("Updating junction to {}...", candidate_dir.display());
    junction_setup::set_junction(junction_path, candidate_dir).expect("Junction failed");

    let config = fetch::fetch_tool_config(category).expect("Could not load tool config");
    env_update::apply(&config.home_var, junction_path, &config.bin_subdir).expect("Env update failed");

    Ok(())
}
use crate::cache;
use anyhow::bail;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::blocking::Client;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::{env, fs};

pub fn run(category: &str, vendor: &str, version: &str) {
    let cache = cache::get();

    let vendors = match cache.data.get(category) {
        Some(v) => v,
        None => {
            eprintln!("No data found for category '{}'.", category);
            std::process::exit(1);
        }
    };

    let entries = match vendors.get(vendor) {
        Some(e) => e,
        None => {
            eprintln!("Unknown vendor '{}' for category '{}'.", vendor, category);
            std::process::exit(1);
        }
    };

    let entry = match entries.iter().find(|e| e.version == version) {
        Some(e) => e,
        None => {
            eprintln!(
                "Version '{}' not found for '{} {}'.",
                version, category, vendor
            );
            std::process::exit(1);
        }
    };

    download(
        &entry.url,
        &entry.checksums.as_ref().unwrap().sha256.as_ref().unwrap(),
    )
    .expect("expected download to occur");
}

fn download(url_to_download: &str, checksum: &str) -> anyhow::Result<()> {
    let client = Client::new();
    let mut resp = client.get(url_to_download).send()?; // resp is now Response

    let total_size = resp
        .content_length()
        .ok_or_else(|| anyhow::anyhow!("No content length"))?;

    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::with_template(
        "{bar:40.cyan/blue} {bytes}/{total_bytes} ({eta})",
    )?);

    let downloads_dir = cauldron_download_path()?;
    let filename = filename_from_url(url_to_download)?;
    let zip_path = downloads_dir.join(filename);

    if zip_path.exists() {
        println!("File already downloaded: {}", zip_path.display());
        verify_checksums(&zip_path.to_str().unwrap(), checksum).expect("checksum should match");
        // TODO : verify checksum here to make sure the file isn't corrupt
        return Ok(());
    }

    println!("Downloading: {}", zip_path.display());

    let mut file = File::create(&zip_path)?;

    let mut downloaded: u64 = 0;
    let mut buffer = [0; 8192];

    loop {
        let n = resp.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        file.write_all(&buffer[..n])?;
        downloaded += n as u64;
        pb.set_position(downloaded);
    }

    pb.finish_with_message("Download complete");
    verify_checksums(&zip_path.to_str().unwrap(), checksum).expect("checksum should match");
    Ok(())
}

fn cauldron_download_path() -> anyhow::Result<PathBuf> {
    let user_profile = env::var("USERPROFILE")?; // get %USERPROFILE%
    let mut path = PathBuf::from(user_profile);
    path.push(".cauldron");
    path.push("downloads");

    // create the directory if it doesn't exist
    fs::create_dir_all(&path)?;

    Ok(path)
}

fn filename_from_url(url: &str) -> anyhow::Result<String> {
    let parts: Vec<&str> = url.split('/').collect();
    let last = parts
        .last()
        .ok_or_else(|| anyhow::anyhow!("URL has no filename segment"))?;
    Ok(last.to_string())
}

fn verify_checksums(path: &str, expected: &str) -> anyhow::Result<()> {
    println!("Verifying checksum: {}", path);
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();

    let mut buffer = [0; 8192];
    while let Ok(n) = file.read(&mut buffer) {
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    let expected_hash = hasher.finalize();
    let input_hash = hex::decode(expected.trim()).expect("Failed to decode");

    if &expected_hash[..] != &input_hash[..] {
        bail!("Checksum verification failed");
    }

    println!("Checksum OK");

    Ok(())
}

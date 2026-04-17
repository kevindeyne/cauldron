use crate::{cache, env_update, fetch, junction_setup, unpack, util};
use anyhow::{bail, Context, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::blocking::Client;
use sha2::{Digest, Sha256, Sha512};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const MAX_CACHED_DOWNLOADS: usize = 2;

// ---- Checksum ----

enum Checksum<'a> {
    Sha256(&'a str),
    Sha512(&'a str),
}

// ---- Progress container ----

struct Bars {
    download: ProgressBar,
    verify: ProgressBar,
    unpack: ProgressBar,
    junction: ProgressBar,
    env: ProgressBar,
    cleanup: ProgressBar,
}

fn create_bars(total: u64) -> (MultiProgress, Bars) {
    let mp = MultiProgress::new();

    let spinner = ProgressStyle::with_template("{spinner} {msg}")
        .unwrap()
        .tick_strings(&["⠋","⠙","⠹","⠸","⠼","⠴","⠦","⠧","⠇","⠏"]);

    let bar = ProgressStyle::with_template(
        "{bar:40.cyan/blue} {bytes}/{total_bytes} ({eta}) {msg}"
    ).unwrap();

    let bars = Bars {
        download: mp.add(ProgressBar::new(total)),
        verify: mp.add(ProgressBar::new_spinner()),
        unpack: mp.add(ProgressBar::new_spinner()),
        junction: mp.add(ProgressBar::new_spinner()),
        env: mp.add(ProgressBar::new_spinner()),
        cleanup: mp.add(ProgressBar::new_spinner()),
    };

    bars.download.set_style(bar);

    for pb in [&bars.verify, &bars.unpack, &bars.junction, &bars.env, &bars.cleanup] {
        pb.set_style(spinner.clone());
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
    }

    (mp, bars)
}

// ---- Public entry (unchanged API) ----

pub fn run(category: &str, vendor: &str, version: &str) {
    let cache = cache::get();

    let entry = cache
        .data
        .get(category)
        .and_then(|vendors| vendors.get(vendor))
        .and_then(|entries| entries.iter().find(|e| e.version == version))
        .unwrap_or_else(|| {
            eprintln!(
                "'{}' '{}' version '{}' not found. Run 'sdk list {}'",
                vendor, version, category, category
            );
            std::process::exit(1);
        });

    let checksum = if let Some(sha256) = entry.checksums.as_ref().and_then(|c| c.sha256.as_ref()) {
        Checksum::Sha256(sha256)
    } else if let Some(sha512) = entry.checksums.as_ref().and_then(|c| c.sha512.as_ref()) {
        Checksum::Sha512(sha512)
    } else {
        eprintln!("No SHA-256 or SHA-512 checksum available.");
        std::process::exit(1);
    };

    let candidate_dir = util::candidates_dir(category, vendor, version);
    let junction_path = util::junction_path(category);

    if let Err(e) = install(
        &entry.url,
        checksum,
        &candidate_dir,
        &junction_path,
        category,
    ) {
        eprintln!("Install failed: {:#}", e);
        std::process::exit(1);
    }

    println!("Done! {} {} {} is now active.", category, vendor, version);
}

// ---- Pipeline ----

fn install(
    url: &str,
    checksum: Checksum,
    candidate_dir: &Path,
    junction_path: &Path,
    category: &str,
) -> Result<()> {
    let downloads_dir = util::cauldron_dir().join("downloads");
    fs::create_dir_all(&downloads_dir)?;

    let filename = url.split('/').last().context("Invalid URL")?;
    let zip_path = downloads_dir.join(filename);

    // Pre-fetch size for progress bar
    let client = Client::new();
    let total = client
        .head(url)
        .send()?
        .content_length()
        .context("Missing content-length")?;

    let (_mp, bars) = create_bars(total);

    // ---- Download ----
    if zip_path.exists() {
        //bars.download.finish_with_message("Already downloaded");
        bars.download.finish_and_clear();
        bars.download.set_message("Already downloaded");
    } else {
        fetch_to_disk(url, &zip_path, &bars.download)?;
    }

    // ---- Cleanup ----
    bars.cleanup.set_message("Evicting old downloads");
    evict_old_downloads(&downloads_dir);
    bars.cleanup.finish_with_message("Cache OK");

    // ---- Verify ----
    bars.verify.set_message("Verifying checksum");
    verify_checksum(&zip_path, checksum)?;
    bars.verify.finish_with_message("Checksum OK");

    // ---- Unpack ----
    bars.unpack.set_message("Unpacking");
    unpack::unpack_zip(zip_path.to_str().unwrap(), candidate_dir)
        .expect("Unpack failed");
    bars.unpack.finish_with_message("Unpacked");

    // ---- Junction ----
    bars.junction.set_message("Linking");
    junction_setup::set_junction(junction_path, candidate_dir)
        .expect("Junction failed");
    bars.junction.finish_with_message("Linked");

    // ---- Env ----
    bars.env.set_message("Updating environment");
    let config = fetch::fetch_tool_config(category)
        .expect("Config load failed");
    env_update::apply(&config.home_var, junction_path, &config.bin_subdir)
        .expect("Env update failed");
    bars.env.finish_with_message("Environment ready");

    Ok(())
}

// ---- Helpers ----

fn fetch_to_disk(url: &str, zip_path: &Path, pb: &ProgressBar) -> Result<()> {
    let client = Client::new();
    let mut resp = client.get(url).send()?;

    let mut file = File::create(zip_path)?;
    let mut buf = [0u8; 8192];
    let mut downloaded = 0;

    loop {
        let n = resp.read(&mut buf)?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        downloaded += n as u64;
        pb.set_position(downloaded);
    }

    //pb.finish_with_message("Download complete");
    pb.finish_and_clear();
    pb.set_message("Download complete");
    Ok(())
}


fn verify_checksum(zip_path: &Path, checksum: Checksum) -> Result<()> {
    let mut file = File::open(zip_path)?;
    let mut buf = [0u8; 8192];

    match checksum {
        Checksum::Sha256(expected) => {
            let mut hasher = Sha256::new();
            loop {
                let n = file.read(&mut buf)?;
                if n == 0 { break; }
                hasher.update(&buf[..n]);
            }
            let actual = hasher.finalize();
            let expected_bytes = hex::decode(expected.trim())?;
            if actual[..] != expected_bytes[..] {
                bail!("SHA-256 checksum mismatch");
            }
        }
        Checksum::Sha512(expected) => {
            let mut hasher = Sha512::new();
            loop {
                let n = file.read(&mut buf)?;
                if n == 0 { break; }
                hasher.update(&buf[..n]);
            }
            let actual = hasher.finalize();
            let expected_bytes = hex::decode(expected.trim())?;
            if actual[..] != expected_bytes[..] {
                bail!("SHA-512 checksum mismatch");
            }
        }
    }

    Ok(())
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

    zips.sort_by(|a, b| b.0.cmp(&a.0));

    for (_, path) in zips.into_iter().skip(MAX_CACHED_DOWNLOADS) {
        let _ = fs::remove_file(path);
    }
}
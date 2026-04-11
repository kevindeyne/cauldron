use std::{env, fs, path::PathBuf};

fn main() {
    let out_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("target")
        .join(env::var("PROFILE").unwrap())  // "debug" or "release"
        .join("cauldron.ps1");

    let src = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("scripts")
        .join("cauldron.ps1");

    fs::copy(&src, &out_dir).expect("Failed to copy cauldron.ps1 to target dir");

    // Re-run build script if the ps1 changes
    println!("cargo:rerun-if-changed=scripts/cauldron.ps1");
}
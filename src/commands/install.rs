use crate::cache;

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
            eprintln!("Version '{}' not found for '{} {}'.", version, category, vendor);
            std::process::exit(1);
        }
    };

    println!("{}", entry.url);
}
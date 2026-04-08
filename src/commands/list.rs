use crate::{cache, util::version_cmp};

pub fn run(category: &str) {
    let cache = cache::get();

    let vendors = match cache.data.get(category) {
        Some(v) => v,
        None => {
            eprintln!("No data found for category '{}'.", category);
            std::process::exit(1);
        }
    };

    let mut rows: Vec<(String, String)> = vendors
        .iter()
        .flat_map(|(vendor, entries)| {
            entries.iter().map(|e| (vendor.clone(), e.version.clone()))
        })
        .collect();

    rows.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| version_cmp(&b.1, &a.1)));

    for (vendor, version) in rows {
        println!("{} {}", vendor, version);
    }
}
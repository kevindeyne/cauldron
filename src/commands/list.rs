use crate::{cache, util::version_cmp};
use comfy_table::{Table, presets::UTF8_FULL};

pub fn run(category: &str) {
    let cache = cache::get();

    let vendors = match cache.data.get(category) {
        Some(v) => v,
        None => {
            eprintln!("No data found for category '{}'.", category);
            std::process::exit(1);
        }
    };

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["Vendor", "Versions"]);

    // Collect → sort → join per vendor
    let mut vendor_rows: Vec<(&String, Vec<String>)> = vendors
        .iter()
        .map(|(vendor, entries)| {
            let mut versions: Vec<String> =
                entries.iter().map(|e| e.version.clone()).collect();

            versions.sort_by(|a, b| version_cmp(b, a)); // descending

            (vendor, versions)
        })
        .collect();

    // Sort vendors alphabetically
    vendor_rows.sort_by(|a, b| a.0.cmp(b.0));

    for (vendor, versions) in vendor_rows {
        let versions_str = versions.join(", ");
        table.add_row(vec![vendor.as_str(), versions_str.as_str()]);
    }

    println!("{table}");
}
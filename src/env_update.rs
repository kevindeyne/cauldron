use std::path::Path;
use winreg::{enums::*, RegKey};

const ENV_KEY: &str = "Environment";

/// Set `home_var` to `junction_path` and ensure `{junction_path}\{bin_subdir}` is in PATH,
/// replacing any existing cauldron-managed entry for this tool.
pub fn apply(home_var: &str, junction_path: &Path, bin_subdir: &str) -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu
        .open_subkey_with_flags(ENV_KEY, KEY_READ | KEY_WRITE)
        .map_err(|e| format!("Cannot open HKCU\\Environment: {}", e))?;

    let junction_str = junction_path
        .to_str()
        .ok_or("Junction path is not valid UTF-8")?;

    // Set HOME var (e.g. JAVA_HOME)
    env.set_value(home_var, &junction_str)
        .map_err(|e| format!("Cannot set {}: {}", home_var, e))?;

    // Verify it was written
    let written_home: String = env.get_value(home_var).unwrap_or_else(|_| "<unreadable>".into());
    println!("  {} = {}", home_var, written_home);

    let new_bin = format!("{}\\{}", junction_str, bin_subdir);

    let current_path: String = env.get_value("PATH").unwrap_or_default();
    println!("  PATH before: {}", current_path);

    let updated_path = update_path(&current_path, &new_bin, junction_str);
    println!("  PATH after:  {}", updated_path);

    env.set_value("PATH", &updated_path)
        .map_err(|e| format!("Cannot set PATH: {}", e))?;

    // Verify PATH was written
    let written_path: String = env.get_value("PATH").unwrap_or_else(|_| "<unreadable>".into());
    println!("  PATH written: {}", written_path);

    broadcast_settings_change();
    println!("  WM_SETTINGCHANGE broadcast sent");

    Ok(())
}

/// Replace any existing cauldron-managed entry for this tool in PATH, or append if absent.
/// Matches on the junction base path so it catches entries from previous installs.
fn update_path(current: &str, new_bin: &str, junction_path: &str) -> String {
    // Normalise to lowercase for comparison
    let junction_lower = junction_path.to_lowercase();

    let mut entries: Vec<&str> = current
        .split(';')
        .filter(|e| !e.is_empty() && !e.to_lowercase().starts_with(&junction_lower))
        .collect();

    entries.insert(0, new_bin);
    entries.join(";")
}

/// Broadcast WM_SETTINGCHANGE so other processes (Explorer, shells) pick up the new env.
fn broadcast_settings_change() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
    };

    let param = "Environment\0"
        .encode_utf16()
        .collect::<Vec<u16>>();

    unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0,
            param.as_ptr() as isize,
            SMTO_ABORTIFHUNG,
            5000,
            std::ptr::null_mut(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::update_path;

    #[test]
    fn prepends_new_entry_when_no_existing() {
        let result = update_path("C:\\Windows\\system32", "C:\\cauldron\\current\\java\\bin", "JAVA_HOME");
        assert!(result.starts_with("C:\\cauldron\\current\\java\\bin"));
        assert!(result.contains("C:\\Windows\\system32"));
    }

    #[test]
    fn replaces_existing_java_home_entry() {
        let current = "C:\\old-java\\bin;C:\\Windows\\system32";
        // Simulate a previous cauldron PATH entry containing "java_home"
        let current = "C:\\cauldron\\current\\java_home\\bin;C:\\Windows\\system32";
        let result = update_path(current, "C:\\cauldron\\current\\java\\bin", "JAVA_HOME");
        assert!(result.starts_with("C:\\cauldron\\current\\java\\bin"));
        assert!(!result.contains("java_home\\bin;C:\\cauldron\\current\\java\\bin")); // no duplicate
        assert!(result.contains("C:\\Windows\\system32"));
    }

    #[test]
    fn does_not_affect_unrelated_entries() {
        let current = "C:\\tools\\maven\\bin;C:\\Windows\\system32";
        let result = update_path(current, "C:\\cauldron\\current\\java\\bin", "JAVA_HOME");
        assert!(result.contains("C:\\tools\\maven\\bin"));
        assert!(result.contains("C:\\Windows\\system32"));
    }
}
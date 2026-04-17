use std::path::Path;
use windows_sys::Win32::Foundation::HWND;
use winreg::{enums::*, RegKey};

const USER_ENV_KEY: &str = "Environment";
const SYSTEM_ENV_KEY: &str = "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment";

// Keywords that identify a Java installation in PATH entries
const JAVA_MARKERS: &[&str] = &[
    "jdk", "jre", "java", "corretto", "adoptium", "temurin", "graalvm", "zulu", "liberica", "semeru",
];

// Keywords that identify a Maven installation in PATH entries
const MAVEN_MARKERS: &[&str] = &[
    "maven", "mvn", "apache-maven",
];

pub fn apply(home_var: &str, junction_path: &Path, bin_subdir: &str) -> Result<(), String> {
    let junction_str = junction_path
        .to_str()
        .ok_or("Junction path is not valid UTF-8")?;

    let new_bin = format!("{}\\{}", junction_str, bin_subdir);

    clean_user_path(&new_bin, junction_str, home_var)?;
    clean_system_path(home_var)?;
    set_home_var(home_var, junction_str)?;
    broadcast_settings_change();

    println!("  {} = {}", home_var, junction_str);
    Ok(())
}

/// Write HOME var to HKCU\Environment.
fn set_home_var(home_var: &str, value: &str) -> Result<(), String> {
    let env = open_user_env(KEY_READ | KEY_WRITE)?;
    env.set_value(home_var, &value)
        .map_err(|e| format!("Cannot set {}: {}", home_var, e))
}

/// Remove any non-cauldron conflicting entries from the user PATH, then prepend new_bin.
fn clean_user_path(new_bin: &str, junction_str: &str, home_var: &str) -> Result<(), String> {
    let env = open_user_env(KEY_READ | KEY_WRITE)?;
    let current: String = env.get_value("PATH").unwrap_or_default();

    let junction_lower = junction_str.to_lowercase();
    let mut removed = vec![];

    let mut entries: Vec<&str> = current
        .split(';')
        .filter(|e| {
            if e.is_empty() { return false; }
            let lower = e.to_lowercase();
            // Keep if it's our cauldron junction
            if lower.starts_with(&junction_lower) { return false; }
            // Remove if it looks like an installation
            if looks_like_installation(home_var, &lower) {
                removed.push(*e);
                return false;
            }
            true
        })
        .collect();

    for r in &removed {
        println!("  Removing old conflicting entry from user PATH: {}", r);
    }

    entries.insert(0, new_bin);
    let updated = entries.join(";");

    env.set_value("PATH", &updated)
        .map_err(|e| format!("Cannot set user PATH: {}", e))?;

    println!("  User PATH updated");
    Ok(())
}

/// Scan system PATH for conflicting entries and attempt removal with privilege escalation if needed.
fn clean_system_path(home_var: &str) -> Result<(), String> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    // Try read-only first to check if there's anything to do
    let env_read = match hklm.open_subkey(SYSTEM_ENV_KEY) {
        Ok(k) => k,
        Err(_) => return Ok(()), // can't read system env, skip
    };

    let current: String = env_read.get_value("PATH").unwrap_or_default();
    let conflicting_entries: Vec<&str> = current
        .split(';')
        .filter(|e| !e.is_empty() && looks_like_installation(home_var, &e.to_lowercase()))
        .collect();

    if conflicting_entries.is_empty() {
        return Ok(());
    }

    println!("  Found conflicting entries in system PATH that conflict with cauldron:");
    for e in &conflicting_entries {
        println!("    {}", e);
    }
    println!("  Attempting to remove them. This requires administrator privileges.");
    println!("  If a UAC prompt appears, please accept it to allow cauldron to clean up the system PATH.");

    // Try to open with write access — this will fail without admin rights
    match hklm.open_subkey_with_flags(SYSTEM_ENV_KEY, KEY_READ | KEY_WRITE) {
        Ok(env_write) => {
            let cleaned: Vec<&str> = current
                .split(';')
                .filter(|e| !e.is_empty() && !looks_like_installation(home_var, &e.to_lowercase()))
                .collect();

            env_write
                .set_value("PATH", &cleaned.join(";"))
                .map_err(|e| format!("Cannot write system PATH: {}", e))?;

            println!("  System PATH cleaned successfully.");
        }
        Err(_) => {
            // Re-launch self as admin via ShellExecuteW with "runas"
            println!("  Elevation required. Launching elevated process to clean system PATH...");
            if let Err(e) = relaunch_as_admin_for_cleanup(&conflicting_entries) {
                println!("  Could not elevate: {}.", e);
                println!("  Please remove the above entries from your system PATH manually.");
            }
        }
    }

    Ok(())
}

/// Spawn an elevated process to remove specific entries from the system PATH.
/// We pass the entries as a semicolon-separated argument to a `--clean-system-path` hidden subcommand.
fn relaunch_as_admin_for_cleanup(entries_to_remove: &[&str]) -> Result<(), String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;

    let exe = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .to_string();

    let args = format!(
        "--clean-system-path \"{}\"",
        entries_to_remove.join(";")
    );

    fn to_wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
    }

    let verb = to_wide("runas");
    let file = to_wide(&exe);
    let params = to_wide(&args);

    let hwnd: HWND = std::ptr::null_mut();

    let result = unsafe {
        ShellExecuteW(
            hwnd,
            verb.as_ptr(),
            file.as_ptr(),
            params.as_ptr(),
            std::ptr::null(),
            SW_HIDE as i32,
        )
    };

    // ShellExecuteW returns > 32 on success
    if result as usize <= 32 {
        return Err(format!("ShellExecuteW returned {}", result as usize));
    }

    Ok(())
}

/// Called when the process is re-launched as admin with `--clean-system-path`.
pub fn clean_system_path_elevated(raw_entries: &str) {
    let entries_to_remove: Vec<String> = raw_entries
        .split(';')
        .map(|s| s.to_lowercase())
        .collect();

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let env = match hklm.open_subkey_with_flags(SYSTEM_ENV_KEY, KEY_READ | KEY_WRITE) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("Elevated process: cannot open system PATH: {}", e);
            return;
        }
    };

    let current: String = env.get_value("PATH").unwrap_or_default();
    let cleaned: Vec<&str> = current
        .split(';')
        .filter(|e| !e.is_empty() && !entries_to_remove.contains(&e.to_lowercase()))
        .collect();

    match env.set_value("PATH", &cleaned.join(";")) {
        Ok(_) => println!("System PATH cleaned successfully."),
        Err(e) => eprintln!("Elevated process: failed to write system PATH: {}", e),
    }

    broadcast_settings_change();
}

fn looks_like_installation(home_var: &str, lower: &str) -> bool {
    let home_lower = home_var.to_lowercase();
    // Always match on the actual home var name (e.g. "java_home", "maven_home")
    if lower.contains(&home_lower) { return true; }
    
    let markers = if home_lower.contains("java") {
        JAVA_MARKERS
    } else if home_lower.contains("maven") {
        MAVEN_MARKERS
    } else {
        &[]
    };
    
    markers.iter().any(|m| lower.contains(m))
}

fn open_user_env(access: u32) -> Result<RegKey, String> {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(USER_ENV_KEY, access)
        .map_err(|e| format!("Cannot open HKCU\\Environment: {}", e))
}

fn broadcast_settings_change() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
    };
    let param: Vec<u16> = "Environment\0".encode_utf16().collect();
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
    use super::*;

    #[test]
    fn detects_java_markers() {
        assert!(looks_like_installation("JAVA_HOME", r"c:\program files\java\jdk-21\bin"));
        assert!(looks_like_installation("JAVA_HOME", r"c:\program files\corretto-21\bin"));
        assert!(looks_like_installation("JAVA_HOME", r"c:\tools\temurin\bin"));
        assert!(!looks_like_installation("JAVA_HOME", r"c:\windows\system32"));
        assert!(!looks_like_installation("JAVA_HOME", r"c:\users\kevin\.cargo\bin"));
    }

    #[test]
    fn detects_maven_markers() {
        assert!(looks_like_installation("MAVEN_HOME", r"c:\tools\apache-maven\bin"));
        assert!(looks_like_installation("MAVEN_HOME", r"c:\program files\maven\bin"));
        assert!(looks_like_installation("MAVEN_HOME", r"c:\mvn\bin"));
        assert!(!looks_like_installation("MAVEN_HOME", r"c:\windows\system32"));
        assert!(!looks_like_installation("MAVEN_HOME", r"c:\users\kevin\.cargo\bin"));
    }

    #[test]
    fn does_not_flag_maven_as_java() {
        assert!(!looks_like_installation("JAVA_HOME", r"c:\tools\apache-maven\bin"));
        assert!(!looks_like_installation("JAVA_HOME", r"c:\windows\system32"));
    }
}
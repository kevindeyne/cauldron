# 🫕Cauldron - WIP
_This is a work in progress and not intended for any production purposes_

Note: Works together with https://github.com/kevindeyne/cauldron-recipes

A lightweight Windows SDK manager written in Rust. Cauldron lets you install and switch between versions of developer tools (Java, Maven, Ant, JMeter, ...) from the command line, with automatic PATH and environment variable management. Inspired by https://sdkman.io/

## Features

- List available vendors and versions for any supported tool
- Download, verify (SHA-256), and unpack SDK distributions
- Manage installations via Windows directory junctions — no copies, instant switching
- Automatically updates `JAVA_HOME` (and equivalent) and `PATH` in the user registry
- Detects and removes conflicting system-level Java entries, with UAC elevation only when needed
- Daily-cached version index fetched from GitHub, with local fallback
- PowerShell wrapper refreshes the current terminal session immediately after install

## Requirements

- Windows 10 or later
- PowerShell 5.1+

## Installation

1. Download or build `cauldron.exe` and `cauldron.ps1` and place them in a directory on your `PATH` (e.g. `C:\tools\cauldron\`).
2. Use `cauldron` (via the `.ps1` wrapper) instead of calling `cauldron.exe` directly so your terminal session is refreshed automatically after installs.

## Usage

```powershell
# List available versions for a tool
cauldron list java

# Install a specific version
cauldron install java corretto 21
cauldron install java corretto 25

# Switching versions is just another install
cauldron install java adoptium 21
```

## How it works

**Version index** — Cauldron fetches a list of available tools and versions from [`kevindeyne/cauldron-recipes`](https://github.com/kevindeyne/cauldron-recipes) on GitHub. The index is cached locally at `~/.cauldron/cache.json` and refreshed at most once per day.

**Install flow** — For each `install` command, Cauldron will:
1. Download the zip to `~/.cauldron/downloads/` (skipped if already present)
2. Verify the SHA-256 checksum
3. Unpack to `~/.cauldron/candidates/{tool}/{vendor}/{version}/`
4. Update the junction at `~/.cauldron/current/{tool}` to point to the new version
5. Set `{TOOL}_HOME` and update `PATH` in `HKCU\Environment`
6. Scan for conflicting system PATH entries and remove them (with UAC prompt if required)
7. Broadcast `WM_SETTINGCHANGE` so other processes pick up the new environment

**Junctions** — Cauldron uses Windows directory junctions rather than copying files. Switching versions is instant and costs no extra disk space.

**PowerShell wrapper** — Because a running terminal inherits a snapshot of the environment at launch, the `.ps1` wrapper re-reads `HKCU` and `HKLM` after a successful install and patches the current session in place. This means `java --version` works immediately without opening a new terminal.

## Building from source

```powershell
cargo build --release
# Outputs: target/release/cauldron.exe
#          target/release/cauldron.ps1
```

The build script (`build.rs`) automatically copies `scripts/cauldron.ps1` into the target directory alongside the binary.

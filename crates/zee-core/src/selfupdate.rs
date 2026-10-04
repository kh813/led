use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GITHUB_REPO: &str = "kh813/zee";

/// Open a URL in default browser.
pub fn open_url(url: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(url).spawn()?;
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open").arg(url).spawn()?;
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd").args(["/c", "start", "", url]).spawn()?;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppType {
    Gui,
    Cli,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    #[serde(default)]
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GithubReleaseResponse {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Clone)]
pub struct ReleaseInfo {
    pub tag_name: String,
    pub version: String,
    pub html_url: String,
    pub asset_name: Option<String>,
    pub asset_url: Option<String>,
    pub asset_size: Option<u64>,
    pub body: Option<String>,
}

/// Query GitHub Releases for the latest published release.
pub fn check_latest(app_type: AppType) -> Result<ReleaseInfo> {
    let api_url = format!("https://api.github.com/repos/{}/releases/latest", GITHUB_REPO);
    
    let resp = ureq::get(&api_url)
        .set("User-Agent", &format!("zee-updater/{}", CURRENT_VERSION))
        .set("Accept", "application/vnd.github+json")
        .timeout(std::time::Duration::from_secs(10))
        .call()
        .context("Failed to connect to GitHub Releases API")?;

    if resp.status() != 200 {
        return Err(anyhow!("GitHub API returned HTTP status {}", resp.status()));
    }

    let payload: GithubReleaseResponse = resp.into_json()
        .context("Failed to parse GitHub release JSON")?;

    let version = payload.tag_name.trim_start_matches('v').to_string();
    let matching_asset = find_matching_asset(&payload.assets, app_type);

    Ok(ReleaseInfo {
        tag_name: payload.tag_name,
        version,
        html_url: payload.html_url,
        asset_name: matching_asset.map(|a| a.name.clone()),
        asset_url: matching_asset.map(|a| a.browser_download_url.clone()),
        asset_size: matching_asset.map(|a| a.size),
        body: payload.body,
    })
}

/// Find matching release asset for the current OS, CPU architecture, and application type.
pub fn find_matching_asset(assets: &[ReleaseAsset], app_type: AppType) -> Option<&ReleaseAsset> {
    find_matching_asset_for_platform(assets, app_type, std::env::consts::OS, std::env::consts::ARCH)
}

/// Find matching release asset for a specific target OS and architecture.
pub fn find_matching_asset_for_platform<'a>(
    assets: &'a [ReleaseAsset],
    app_type: AppType,
    os: &str,
    arch: &str,
) -> Option<&'a ReleaseAsset> {
    let matches_app_type = |name: &str| match app_type {
        AppType::Gui => name.starts_with("zeeg-"),
        AppType::Cli => name.starts_with("zee-") && !name.starts_with("zeeg-"),
    };

    let is_arm64 = arch == "aarch64" || arch == "arm64";
    let matches_arch = |name: &str| {
        if is_arm64 {
            name.contains("arm64") || name.contains("aarch64")
        } else {
            name.contains("x64") || name.contains("x86_64")
        }
    };

    for asset in assets {
        let name = asset.name.to_lowercase();
        let is_archive = name.ends_with(".zip") || name.ends_with(".tar.gz");

        if !matches_app_type(&name) || !is_archive || !matches_arch(&name) {
            continue;
        }

        if (os == "macos" && name.contains("macos"))
            || (os == "linux" && name.contains("linux"))
            || (os == "windows" && name.contains("windows"))
        {
            return Some(asset);
        }
    }

    None
}

/// Compare two semantic version strings (e.g. "0.1.0" and "0.1.1").
pub fn is_newer(current: &str, latest: &str) -> bool {
    let c_parts = parse_version_numbers(current);
    let l_parts = parse_version_numbers(latest);

    match (c_parts, l_parts) {
        (Some(c), Some(l)) => {
            let max_len = c.len().max(l.len());
            for i in 0..max_len {
                let cv = c.get(i).copied().unwrap_or(0);
                let lv = l.get(i).copied().unwrap_or(0);
                if lv != cv {
                    return lv > cv;
                }
            }
            false
        }
        _ => latest != current && !latest.is_empty(),
    }
}

fn parse_version_numbers(v: &str) -> Option<Vec<u64>> {
    let cleaned = v.trim().trim_start_matches('v');
    let parts: Result<Vec<u64>, _> = cleaned.split('.').map(|s| s.parse::<u64>()).collect();
    parts.ok()
}

/// Download a file from URL to memory.
pub fn download_file(url: &str) -> Result<Vec<u8>> {
    let resp = ureq::get(url)
        .set("User-Agent", &format!("zee-updater/{}", CURRENT_VERSION))
        .timeout(std::time::Duration::from_secs(60))
        .call()
        .context("Failed to download update package")?;

    if resp.status() != 200 {
        return Err(anyhow!("Failed to download update: HTTP status {}", resp.status()));
    }

    let mut reader = resp.into_reader();
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).context("Failed to read update data")?;
    Ok(bytes)
}

/// Apply update from downloaded archive in place.
pub fn apply_update(asset_url: &str, app_type: AppType) -> Result<()> {
    let archive_bytes = download_file(asset_url)?;
    
    let current_exe = std::env::current_exe()
        .context("Could not determine currently running executable path")?;
    let current_exe = current_exe.canonicalize().unwrap_or(current_exe);

    #[cfg(target_os = "macos")]
    {
        if app_type == AppType::Gui {
            return apply_macos_gui_bundle(&archive_bytes, &current_exe);
        } else {
            return apply_unix_binary(&archive_bytes, &current_exe, "zee");
        }
    }

    #[cfg(target_os = "linux")]
    {
        let bin_name = if app_type == AppType::Gui { "zeeg" } else { "zee" };
        return apply_unix_binary(&archive_bytes, &current_exe, bin_name);
    }

    #[cfg(target_os = "windows")]
    {
        let bin_name = if app_type == AppType::Gui { "zeeg.exe" } else { "zee.exe" };
        return apply_windows_binary(&archive_bytes, &current_exe, bin_name);
    }

    #[allow(unreachable_code)]
    Err(anyhow!("Self-update is not supported on this platform"))
}

#[cfg(target_os = "macos")]
fn apply_macos_gui_bundle(zip_bytes: &[u8], current_exe: &Path) -> Result<()> {
    // Find .app bundle path from executable path
    let exe_str = current_exe.to_string_lossy();
    let marker = ".app/";
    let app_path = if let Some(idx) = exe_str.find(marker) {
        PathBuf::from(&exe_str[..idx + marker.len() - 1])
    } else {
        return Err(anyhow!(
            "Not running from an installed macOS .app bundle ({}) - please update manually from the releases page",
            exe_str
        ));
    };

    let staging_dir = tempfile_staging_dir("zee-update")?;
    let new_app_path = extract_zip_app_bundle(zip_bytes, &staging_dir)?;

    // Spawn detached helper script to swap .app and relaunch
    let script = r#"set -e
sleep 1
rm -rf "$1"
mv "$2" "$1"
xattr -cr "$1" 2>/dev/null || true
codesign --force --deep --sign - "$1" 2>/dev/null || true
rm -rf "$3"
open "$1"
"#;
    let child = Command::new("/bin/sh")
        .arg("-c")
        .arg(script)
        .arg("zee-updater")
        .arg(&app_path)
        .arg(&new_app_path)
        .arg(&staging_dir)
        .spawn()
        .context("Failed to spawn macOS update helper process")?;

    // Let child process detach
    drop(child);
    Ok(())
}

fn extract_zip_app_bundle(zip_bytes: &[u8], dest_dir: &Path) -> Result<PathBuf> {
    use std::io::Cursor;
    let reader = Cursor::new(zip_bytes);
    let mut zip = zip::ZipArchive::new(reader).context("Failed to read ZIP archive")?;

    let mut app_dir_name: Option<String> = None;

    for i in 0..zip.len() {
        let mut file = zip.by_index(i).context("Failed to access file in ZIP")?;
        let raw_name = file.name().to_string();
        
        // Find the root .app folder name
        if let Some(first_component) = raw_name.split('/').next() {
            if first_component.ends_with(".app") {
                app_dir_name = Some(first_component.to_string());
            }
        }

        let outpath = dest_dir.join(&raw_name);

        if file.is_dir() {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                fs::create_dir_all(p)?;
            }
            let mut outfile = File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;
            
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = file.unix_mode() {
                    let _ = fs::set_permissions(&outpath, fs::Permissions::from_mode(mode));
                }
            }
        }
    }

    let app_name = app_dir_name.ok_or_else(|| anyhow!("ZIP archive did not contain a .app bundle"))?;
    Ok(dest_dir.join(app_name))
}

#[cfg(unix)]
fn apply_unix_binary(archive_bytes: &[u8], current_exe: &Path, target_bin_name: &str) -> Result<()> {
    let staging_dir = tempfile_staging_dir("zee-update")?;
    let new_bin_path = staging_dir.join(target_bin_name);

    // Extract binary from zip or tar.gz
    if archive_bytes.starts_with(b"PK") {
        extract_zip_binary(archive_bytes, target_bin_name, &new_bin_path)?;
    } else {
        extract_tar_gz_binary(archive_bytes, target_bin_name, &new_bin_path)?;
    }

    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&new_bin_path, fs::Permissions::from_mode(0o755))
        .context("Failed to set executable permissions on updated binary")?;

    let script = r#"set -e
sleep 0.5
rm -f "$1"
mv "$2" "$1"
chmod +x "$1"
xattr -d com.apple.quarantine "$1" 2>/dev/null || true
codesign --force --sign - "$1" 2>/dev/null || true
rm -rf "$3"
"#;
    let child = Command::new("/bin/sh")
        .arg("-c")
        .arg(script)
        .arg("zee-updater")
        .arg(current_exe)
        .arg(&new_bin_path)
        .arg(&staging_dir)
        .spawn()
        .context("Failed to spawn update helper script")?;

    drop(child);
    Ok(())
}

fn extract_tar_gz_binary(tar_gz_bytes: &[u8], target_name: &str, dest_file: &Path) -> Result<()> {
    use flate2::read::GzDecoder;
    use std::io::Cursor;
    use tar::Archive;

    let tar_data = GzDecoder::new(Cursor::new(tar_gz_bytes));
    let mut archive = Archive::new(tar_data);

    for entry in archive.entries().context("Failed to read tar archive entries")? {
        let mut entry = entry.context("Failed to read tar archive entry")?;
        let path = entry.path().context("Invalid entry path in tar archive")?;
        
        if let Some(file_name) = path.file_name() {
            if file_name == target_name {
                let mut out = File::create(dest_file)
                    .context("Failed to create destination file for extracted binary")?;
                io::copy(&mut entry, &mut out)?;
                return Ok(());
            }
        }
    }

    Err(anyhow!("Binary '{}' not found in update archive", target_name))
}

#[cfg(target_os = "windows")]
fn apply_windows_binary(zip_bytes: &[u8], current_exe: &Path, target_bin_name: &str) -> Result<()> {
    let staging_dir = tempfile_staging_dir("zee-update")?;
    let new_bin_path = staging_dir.join(target_bin_name);

    extract_zip_binary(zip_bytes, target_bin_name, &new_bin_path)?;

    // On Windows, use cmd /c with timeout and move to overwrite locked executable
    let script = format!(
        "timeout /t 1 /nobreak >nul & move /y \"{}\" \"{}\" >nul & rmdir /s /q \"{}\" >nul",
        new_bin_path.display(),
        current_exe.display(),
        staging_dir.display()
    );

    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    const DETACHED_PROCESS: u32 = 0x00000008;

    let mut child = Command::new("cmd")
        .args(["/c", &script])
        .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
        .spawn()
        .context("Failed to spawn Windows update helper process")?;

    drop(child);
    Ok(())
}

#[allow(dead_code)]
fn extract_zip_binary(zip_bytes: &[u8], target_name: &str, dest_file: &Path) -> Result<()> {
    use std::io::Cursor;
    let reader = Cursor::new(zip_bytes);
    let mut zip = zip::ZipArchive::new(reader).context("Failed to read ZIP archive")?;

    for i in 0..zip.len() {
        let mut file = zip.by_index(i).context("Failed to access file in ZIP")?;
        let raw_name = file.name();
        
        if raw_name.ends_with(target_name) {
            let mut out = File::create(dest_file)
                .context("Failed to create destination file for extracted binary")?;
            io::copy(&mut file, &mut out)?;
            return Ok(());
        }
    }

    Err(anyhow!("Binary '{}' not found in update ZIP archive", target_name))
}

fn tempfile_staging_dir(prefix: &str) -> Result<PathBuf> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("{}-{}-{}", prefix, std::process::id(), timestamp));
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        // Test updating from older (e.g. 2nd newest 0.0.9 / 0.1.0) to latest
        assert!(is_newer("0.0.9", "0.1.0"));
        assert!(is_newer("0.1.0", "0.1.1"));
        assert!(is_newer("0.1.0", "0.2.0"));
        assert!(is_newer("0.1.0", "1.0.0"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.1", "0.1.0"));
        assert!(!is_newer("1.0.0", "0.9.9"));
        assert!(is_newer("v0.1.0", "v0.1.1"));
    }

    #[test]
    fn test_all_platforms_asset_resolution() {
        let release_assets = vec![
            ReleaseAsset { name: "zeeg-macos-arm64.zip".to_string(), browser_download_url: "https://example.com/zeeg-macos-arm64.zip".to_string(), size: 1000 },
            ReleaseAsset { name: "zee-macos-arm64.tar.gz".to_string(), browser_download_url: "https://example.com/zee-macos-arm64.tar.gz".to_string(), size: 2000 },
            ReleaseAsset { name: "zeeg-linux-x64.tar.gz".to_string(), browser_download_url: "https://example.com/zeeg-linux-x64.tar.gz".to_string(), size: 3000 },
            ReleaseAsset { name: "zee-linux-x64.tar.gz".to_string(), browser_download_url: "https://example.com/zee-linux-x64.tar.gz".to_string(), size: 4000 },
            ReleaseAsset { name: "zeeg-linux-arm64.tar.gz".to_string(), browser_download_url: "https://example.com/zeeg-linux-arm64.tar.gz".to_string(), size: 5000 },
            ReleaseAsset { name: "zee-linux-arm64.tar.gz".to_string(), browser_download_url: "https://example.com/zee-linux-arm64.tar.gz".to_string(), size: 6000 },
            ReleaseAsset { name: "zeeg-windows-x64.zip".to_string(), browser_download_url: "https://example.com/zeeg-windows-x64.zip".to_string(), size: 7000 },
            ReleaseAsset { name: "zee-windows-x64.zip".to_string(), browser_download_url: "https://example.com/zee-windows-x64.zip".to_string(), size: 8000 },
            ReleaseAsset { name: "zeeg-windows-arm64.zip".to_string(), browser_download_url: "https://example.com/zeeg-windows-arm64.zip".to_string(), size: 9000 },
            ReleaseAsset { name: "zee-windows-arm64.zip".to_string(), browser_download_url: "https://example.com/zee-windows-arm64.zip".to_string(), size: 10000 },
        ];

        // macOS arm64
        let mac_gui = find_matching_asset_for_platform(&release_assets, AppType::Gui, "macos", "aarch64").unwrap();
        assert_eq!(mac_gui.name, "zeeg-macos-arm64.zip");
        let mac_cli = find_matching_asset_for_platform(&release_assets, AppType::Cli, "macos", "aarch64").unwrap();
        assert_eq!(mac_cli.name, "zee-macos-arm64.tar.gz");

        // Linux x64
        let lin_gui_x64 = find_matching_asset_for_platform(&release_assets, AppType::Gui, "linux", "x86_64").unwrap();
        assert_eq!(lin_gui_x64.name, "zeeg-linux-x64.tar.gz");
        let lin_cli_x64 = find_matching_asset_for_platform(&release_assets, AppType::Cli, "linux", "x86_64").unwrap();
        assert_eq!(lin_cli_x64.name, "zee-linux-x64.tar.gz");

        // Linux arm64
        let lin_gui_arm64 = find_matching_asset_for_platform(&release_assets, AppType::Gui, "linux", "aarch64").unwrap();
        assert_eq!(lin_gui_arm64.name, "zeeg-linux-arm64.tar.gz");
        let lin_cli_arm64 = find_matching_asset_for_platform(&release_assets, AppType::Cli, "linux", "aarch64").unwrap();
        assert_eq!(lin_cli_arm64.name, "zee-linux-arm64.tar.gz");

        // Windows x64
        let win_gui_x64 = find_matching_asset_for_platform(&release_assets, AppType::Gui, "windows", "x86_64").unwrap();
        assert_eq!(win_gui_x64.name, "zeeg-windows-x64.zip");
        let win_cli_x64 = find_matching_asset_for_platform(&release_assets, AppType::Cli, "windows", "x86_64").unwrap();
        assert_eq!(win_cli_x64.name, "zee-windows-x64.zip");

        // Windows arm64
        let win_gui_arm64 = find_matching_asset_for_platform(&release_assets, AppType::Gui, "windows", "aarch64").unwrap();
        assert_eq!(win_gui_arm64.name, "zeeg-windows-arm64.zip");
        let win_cli_arm64 = find_matching_asset_for_platform(&release_assets, AppType::Cli, "windows", "aarch64").unwrap();
        assert_eq!(win_cli_arm64.name, "zee-windows-arm64.zip");
    }

    #[test]
    fn test_tar_gz_extraction() {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use tar::Builder;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        {
            let mut tar = Builder::new(&mut encoder);
            let test_content = b"fake binary executable content for zee v0.1.1";
            let mut header = tar::Header::new_gnu();
            header.set_path("zee").unwrap();
            header.set_size(test_content.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            tar.append(&header, &test_content[..]).unwrap();
            tar.finish().unwrap();
        }
        let tar_gz_bytes = encoder.finish().unwrap();

        let temp_dir = tempfile_staging_dir("test-tar-extract").unwrap();
        let dest_file = temp_dir.join("zee");

        extract_tar_gz_binary(&tar_gz_bytes, "zee", &dest_file).unwrap();
        assert!(dest_file.exists());
        let read_back = fs::read(&dest_file).unwrap();
        assert_eq!(read_back, b"fake binary executable content for zee v0.1.1");
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_zip_app_bundle_extraction() {
        use zip::write::{SimpleFileOptions, ZipWriter};
        use std::io::{Cursor, Write};

        let mut buf = Vec::new();
        {
            let mut zip = ZipWriter::new(Cursor::new(&mut buf));
            let options = SimpleFileOptions::default();
            
            zip.add_directory("zee.app", options).unwrap();
            zip.add_directory("zee.app/Contents", options).unwrap();
            zip.add_directory("zee.app/Contents/MacOS", options).unwrap();
            zip.start_file("zee.app/Contents/MacOS/zeeg", options).unwrap();
            zip.write_all(b"zeeg macos gui binary").unwrap();
            zip.finish().unwrap();
        }

        let temp_dir = tempfile_staging_dir("test-zip-app-extract").unwrap();
        let app_dir = extract_zip_app_bundle(&buf, &temp_dir).unwrap();
        assert!(app_dir.exists());
        assert_eq!(app_dir.file_name().unwrap(), "zee.app");
        assert!(app_dir.join("Contents/MacOS/zeeg").exists());
        let content = fs::read(app_dir.join("Contents/MacOS/zeeg")).unwrap();
        assert_eq!(content, b"zeeg macos gui binary");
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_zip_windows_binary_extraction() {
        use zip::write::{SimpleFileOptions, ZipWriter};
        use std::io::{Cursor, Write};

        let mut buf = Vec::new();
        {
            let mut zip = ZipWriter::new(Cursor::new(&mut buf));
            let options = SimpleFileOptions::default();
            zip.start_file("zee.exe", options).unwrap();
            zip.write_all(b"windows binary data").unwrap();
            zip.finish().unwrap();
        }

        let temp_dir = tempfile_staging_dir("test-zip-win-extract").unwrap();
        let dest = temp_dir.join("zee.exe");
        extract_zip_binary(&buf, "zee.exe", &dest).unwrap();
        assert!(dest.exists());
        assert_eq!(fs::read(&dest).unwrap(), b"windows binary data");
        let _ = fs::remove_dir_all(&temp_dir);
    }
}

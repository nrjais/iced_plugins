//! macOS-specific installation functionality

use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::process::Command;
use tokio::time::{Duration, sleep};

/// Install the update on macOS
pub async fn install(file_path: PathBuf) -> Result<(), String> {
    let extension = file_path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| "Unknown file type".to_string())?;

    match extension {
        "dmg" => install_dmg(file_path).await,
        "gz" if file_path.to_string_lossy().ends_with(".tar.gz") => install_tar_gz(file_path).await,
        "zip" => install_zip(file_path).await,
        _ => Err(format!("Unsupported file type: {}", extension)),
    }
}

/// Install from DMG file
async fn install_dmg(dmg_path: PathBuf) -> Result<(), String> {
    let volume_path = mount_dmg(&dmg_path).await?;

    let copy_result = find_and_copy_app(&volume_path).await;

    unmount_dmg_with_retry(&volume_path).await;

    copy_result
}

/// Mount a DMG file and return the volume path
async fn mount_dmg(dmg_path: &Path) -> Result<String, String> {
    let mount_output = Command::new("hdiutil")
        .args(["attach", "-nobrowse", "-readonly"])
        .arg(dmg_path)
        .output()
        .await
        .map_err(|e| format!("Failed to mount DMG: {}", e))?;

    if !mount_output.status.success() {
        let stderr = String::from_utf8_lossy(&mount_output.stderr);
        return Err(format!("Failed to mount DMG: {}", stderr));
    }

    parse_volume_path(&mount_output.stdout)
}

/// Parse the volume path from hdiutil output
fn parse_volume_path(output: &[u8]) -> Result<String, String> {
    let mount_info = String::from_utf8_lossy(output);

    let volume_path = mount_info
        .lines()
        .rev()
        .find_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }

            let parts: Vec<&str> = line.split('\t').collect();
            for part in parts.iter().rev() {
                let trimmed_part = part.trim();
                if trimmed_part.starts_with("/Volumes/") {
                    return Some(trimmed_part.to_string());
                }
            }
            None
        })
        .ok_or_else(|| {
            format!(
                "Failed to find mount point in hdiutil output. Output was:\n{}",
                mount_info
            )
        })?;

    if !PathBuf::from(&volume_path).exists() {
        return Err(format!(
            "Mount point '{}' does not exist. Full output:\n{}",
            volume_path, mount_info
        ));
    }

    Ok(volume_path)
}

/// Find the .app bundle in a volume and copy it to /Applications
async fn find_and_copy_app(volume_path: &str) -> Result<(), String> {
    let app_bundle = find_app_bundle(Path::new(volume_path)).await?;
    copy_to_applications(&app_bundle).await
}

/// Copy an app bundle to /Applications
async fn copy_to_applications(app_bundle: &fs::DirEntry) -> Result<(), String> {
    let app_name = app_bundle.file_name();
    let dest = PathBuf::from("/Applications").join(&app_name);

    let needs_auth = if dest.exists() {
        std::fs::metadata(&dest)
            .map(|metadata| {
                use std::os::unix::fs::PermissionsExt;
                let mode = metadata.permissions().mode();

                metadata.uid() != unsafe { libc::getuid() } || (mode & 0o200) == 0
            })
            .unwrap_or(true)
    } else {
        std::fs::metadata("/Applications")
            .map(|metadata| {
                use std::os::unix::fs::PermissionsExt;
                let mode = metadata.permissions().mode();
                (mode & 0o200) == 0
            })
            .unwrap_or(true)
    };

    if needs_auth {
        copy_with_authentication(&app_bundle.path(), &dest).await
    } else {
        copy_without_authentication(&app_bundle.path(), &dest).await
    }
}

async fn copy_without_authentication(source: &Path, dest: &Path) -> Result<(), String> {
    let copy_output = Command::new("ditto")
        .arg(source)
        .arg(dest)
        .output()
        .await
        .map_err(|e| format!("Failed to copy app: {}", e))?;

    if copy_output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&copy_output.stderr);

        if stderr.contains("Permission denied") {
            copy_with_authentication(source, dest).await
        } else {
            Err(format!("Failed to copy app to Applications: {}", stderr))
        }
    }
}

async fn copy_with_authentication(source: &Path, dest: &Path) -> Result<(), String> {
    let source_str = source.to_string_lossy();
    let dest_str = dest.to_string_lossy();

    let copy_script = format!(
        r#"do shell script "ditto '{}' '{}'" with administrator privileges"#,
        source_str, dest_str
    );

    let copy_output = Command::new("osascript")
        .args(["-e", &copy_script])
        .output()
        .await
        .map_err(|e| format!("Failed to copy app with authentication: {}", e))?;

    if copy_output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&copy_output.stderr);
        if stderr.contains("User canceled") {
            Err("User canceled authentication".to_string())
        } else {
            Err(format!("Failed to copy app to Applications: {}", stderr))
        }
    }
}

/// Unmount a DMG with retry logic
async fn unmount_dmg_with_retry(volume_path: &str) {
    let _ = Command::new("sync").output().await;

    sleep(Duration::from_millis(500)).await;

    let mut detach_success = false;
    for attempt in 1..=3 {
        let detach_result = Command::new("hdiutil")
            .args(["detach", volume_path])
            .output()
            .await;

        match detach_result {
            Ok(output) if output.status.success() => {
                detach_success = true;
                break;
            }
            Ok(output) => {
                if attempt < 3 {
                    sleep(Duration::from_millis(500)).await;
                } else {
                    eprintln!(
                        "Warning: Failed to detach DMG after {} attempts: {}",
                        attempt,
                        String::from_utf8_lossy(&output.stderr)
                    );
                    eprintln!("Attempting force detach...");

                    let force_result = Command::new("hdiutil")
                        .args(["detach", "-force", volume_path])
                        .output()
                        .await;

                    if let Ok(force_output) = force_result {
                        if force_output.status.success() {
                            detach_success = true;
                        } else {
                            eprintln!(
                                "Warning: Force detach also failed: {}",
                                String::from_utf8_lossy(&force_output.stderr)
                            );
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Warning: Failed to execute hdiutil detach: {}", e);
                break;
            }
        }
    }

    if !detach_success {
        eprintln!(
            "Warning: DMG '{}' may still be mounted. You may need to manually unmount it.",
            volume_path
        );
    }
}

/// Install from tar.gz file
async fn install_tar_gz(tar_gz_path: PathBuf) -> Result<(), String> {
    let extract_dir = tar_gz_path
        .parent()
        .ok_or_else(|| "Invalid tar.gz path".to_string())?;

    extract_tar_gz(&tar_gz_path, extract_dir).await?;

    let app_bundle = find_app_bundle(extract_dir).await?;
    copy_to_applications(&app_bundle).await
}

/// Extract a tar.gz file
async fn extract_tar_gz(tar_gz_path: &Path, extract_dir: &Path) -> Result<(), String> {
    let output = Command::new("tar")
        .args(["-xzf"])
        .arg(tar_gz_path)
        .arg("-C")
        .arg(extract_dir)
        .output()
        .await
        .map_err(|e| format!("Failed to extract tar.gz: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Failed to extract tar.gz: {}", stderr))
    }
}

/// Install from zip file
async fn install_zip(zip_path: PathBuf) -> Result<(), String> {
    let extract_dir = zip_path
        .parent()
        .ok_or_else(|| "Invalid zip path".to_string())?;

    extract_zip(&zip_path, extract_dir).await?;

    let app_bundle = find_app_bundle(extract_dir).await?;
    copy_to_applications(&app_bundle).await
}

/// Extract a zip file
async fn extract_zip(zip_path: &Path, extract_dir: &Path) -> Result<(), String> {
    let output = Command::new("unzip")
        .args(["-o", "-q"])
        .arg(zip_path)
        .arg("-d")
        .arg(extract_dir)
        .output()
        .await
        .map_err(|e| format!("Failed to extract zip: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Failed to extract zip: {}", stderr))
    }
}

/// Find .app bundle in a directory
async fn find_app_bundle(dir_path: &Path) -> Result<fs::DirEntry, String> {
    let mut read_dir = fs::read_dir(dir_path)
        .await
        .map_err(|e| format!("Failed to read directory '{}': {}", dir_path.display(), e))?;

    while let Some(entry) = read_dir.next_entry().await.transpose() {
        if let Ok(entry) = entry
            && entry.path().extension().and_then(|e| e.to_str()) == Some("app")
        {
            return Ok(entry);
        }
    }

    Err(format!("No .app bundle found in '{}'", dir_path.display()))
}

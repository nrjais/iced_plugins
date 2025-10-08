use std::path::PathBuf;

use tokio::process::Command;

pub async fn install_deb(deb_path: PathBuf) -> Result<(), String> {
    let output = Command::new("pkexec")
        .args(["dpkg", "-i"])
        .arg(&deb_path)
        .output()
        .await
        .map_err(|e| format!("Failed to install .deb: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "Failed to install .deb package: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

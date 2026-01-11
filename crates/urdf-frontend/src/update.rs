//! Update checker module
//!
//! Checks for updates from GitHub releases at application startup.

use std::sync::Arc;

use parking_lot::Mutex;

/// Current application version from Cargo.toml
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// GitHub repository owner
const REPO_OWNER: &str = "NOPLAB";

/// GitHub repository name
const REPO_NAME: &str = "rk";

/// Update check result
#[derive(Debug, Clone, Default)]
pub enum UpdateStatus {
    /// Update check in progress
    #[default]
    Checking,
    /// No update available (current version is latest)
    UpToDate,
    /// Update available
    UpdateAvailable {
        latest_version: String,
        release_url: String,
    },
    /// Failed to check for updates
    CheckFailed(String),
}

/// Shared update status
pub type SharedUpdateStatus = Arc<Mutex<UpdateStatus>>;

/// Create a new shared update status
pub fn create_update_status() -> SharedUpdateStatus {
    Arc::new(Mutex::new(UpdateStatus::Checking))
}

/// Check for updates in the background (native only)
#[cfg(not(target_arch = "wasm32"))]
pub fn check_for_updates(status: SharedUpdateStatus) {
    std::thread::spawn(move || {
        let result = check_latest_release();
        *status.lock() = result;
    });
}

/// Check the latest release from GitHub
#[cfg(not(target_arch = "wasm32"))]
fn check_latest_release() -> UpdateStatus {
    use semver::Version;

    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        REPO_OWNER, REPO_NAME
    );

    let response = match ureq::get(&url)
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", concat!("rk/", env!("CARGO_PKG_VERSION")))
        .call()
    {
        Ok(resp) => resp,
        Err(e) => {
            tracing::warn!("Failed to check for updates: {}", e);
            return UpdateStatus::CheckFailed(e.to_string());
        }
    };

    let body: serde_json::Value = match response.into_body().read_json() {
        Ok(json) => json,
        Err(e) => {
            tracing::warn!("Failed to parse update response: {}", e);
            return UpdateStatus::CheckFailed(e.to_string());
        }
    };

    let tag_name = match body["tag_name"].as_str() {
        Some(tag) => tag,
        None => {
            tracing::warn!("No tag_name in release response");
            return UpdateStatus::CheckFailed("No tag_name in response".to_string());
        }
    };

    let html_url = body["html_url"]
        .as_str()
        .unwrap_or("https://github.com/NOPLAB/rk/releases")
        .to_string();

    // Parse version (remove 'v' prefix if present)
    let latest_version_str = tag_name.trim_start_matches('v');
    let current_version_str = CURRENT_VERSION.trim_start_matches('v');

    let latest_version = match Version::parse(latest_version_str) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                "Failed to parse latest version '{}': {}",
                latest_version_str,
                e
            );
            return UpdateStatus::CheckFailed(format!("Invalid version: {}", e));
        }
    };

    let current_version = match Version::parse(current_version_str) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                "Failed to parse current version '{}': {}",
                current_version_str,
                e
            );
            return UpdateStatus::CheckFailed(format!("Invalid current version: {}", e));
        }
    };

    if latest_version > current_version {
        tracing::info!(
            "Update available: {} -> {}",
            current_version,
            latest_version
        );
        UpdateStatus::UpdateAvailable {
            latest_version: latest_version_str.to_string(),
            release_url: html_url,
        }
    } else {
        tracing::debug!(
            "No update available (current: {}, latest: {})",
            current_version,
            latest_version
        );
        UpdateStatus::UpToDate
    }
}

/// No-op for WASM builds
#[cfg(target_arch = "wasm32")]
pub fn check_for_updates(_status: SharedUpdateStatus) {
    // WASM builds don't check for updates
}

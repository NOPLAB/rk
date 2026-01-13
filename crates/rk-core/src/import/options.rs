//! Import options and ROS package discovery

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::mesh::StlUnit;

/// Import options for URDF loading
#[derive(Debug, Clone)]
pub struct ImportOptions {
    /// Base directory for resolving relative mesh paths
    pub base_dir: PathBuf,
    /// Unit for imported STL meshes (URDF typically uses meters)
    pub stl_unit: StlUnit,
    /// Default material color if not specified
    pub default_color: [f32; 4],
    /// Package path mappings for resolving package:// URIs
    /// Maps package name to its root directory
    pub package_paths: HashMap<String, PathBuf>,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            base_dir: PathBuf::from("."),
            stl_unit: StlUnit::Meters,
            default_color: [0.7, 0.7, 0.7, 1.0],
            package_paths: HashMap::new(),
        }
    }
}

impl ImportOptions {
    /// Create import options with package paths auto-discovered from ROS environment
    pub fn with_ros_packages() -> Self {
        Self {
            package_paths: discover_ros_packages(),
            ..Self::default()
        }
    }

    /// Add a package path mapping
    pub fn add_package_path(&mut self, package_name: impl Into<String>, path: impl Into<PathBuf>) {
        self.package_paths.insert(package_name.into(), path.into());
    }
}

/// Discover ROS packages from environment variables
pub fn discover_ros_packages() -> HashMap<String, PathBuf> {
    let mut packages = HashMap::new();

    // Try ROS_PACKAGE_PATH (ROS1 style, colon-separated)
    if let Ok(ros_package_path) = std::env::var("ROS_PACKAGE_PATH") {
        for path_str in ros_package_path.split(':') {
            let path = Path::new(path_str);
            if path.is_dir() {
                discover_packages_in_dir(path, &mut packages);
            }
        }
    }

    // Try AMENT_PREFIX_PATH (ROS2 style, colon-separated)
    if let Ok(ament_prefix_path) = std::env::var("AMENT_PREFIX_PATH") {
        for path_str in ament_prefix_path.split(':') {
            let share_path = Path::new(path_str).join("share");
            if share_path.is_dir() {
                discover_packages_in_dir(&share_path, &mut packages);
            }
        }
    }

    // Try COLCON_PREFIX_PATH (colcon workspaces)
    if let Ok(colcon_prefix_path) = std::env::var("COLCON_PREFIX_PATH") {
        for path_str in colcon_prefix_path.split(':') {
            let share_path = Path::new(path_str).join("share");
            if share_path.is_dir() {
                discover_packages_in_dir(&share_path, &mut packages);
            }
        }
    }

    packages
}

/// Discover packages in a directory (looks for package.xml files)
fn discover_packages_in_dir(dir: &Path, packages: &mut HashMap<String, PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Check if this directory contains a package.xml
                let package_xml = path.join("package.xml");
                if package_xml.exists()
                    && let Some(name) = path.file_name().and_then(|n| n.to_str())
                {
                    packages.insert(name.to_string(), path);
                }
            }
        }
    }
}

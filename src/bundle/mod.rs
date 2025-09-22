mod category;
mod common;
mod ios_bundle;
mod linux;
mod msi_bundle;
mod osx_bundle;
mod settings;
pub mod target_info;
mod wxsmsi_bundle;

pub use self::common::{print_error, print_finished};
pub use self::settings::{BuildArtifact, PackageType, Settings};
use std::path::PathBuf;

pub fn bundle_project(
    settings: Settings,
    package_types: Vec<PackageType>,
) -> crate::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for package_type in package_types {
        let mut results = package_type.bundle_project(&settings)?;
        paths.append(&mut results);
    }
    Ok(paths)
}

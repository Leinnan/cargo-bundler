use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use cargo_metadata::{Metadata, MetadataCommand, Package, TargetKind};
use serde_json::Value;
use target_build_utils::TargetInfo;

use crate::{
    Cli,
    bundle::{BuildArtifact, PackageType, common::print_warning, metadata::BundleSettings},
};

#[derive(Clone, Debug)]
pub struct BundleTargetInfo {
    pub target_info: Option<TargetInfo>,
    pub target_triple: Option<String>,
    pub package_type: PackageType,
    project_out_directory: PathBuf,
    pub profile: String,
    pub package: Package,
}

impl BundleTargetInfo {
    pub fn get_target_dir(&self, build_artifact: &BuildArtifact) -> PathBuf {
        let mut cargo = std::process::Command::new(
            std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo")),
        );
        cargo.args(["metadata", "--no-deps", "--format-version", "1"]);

        let target_dir = cargo.output().ok().and_then(|output| {
            let json_string = String::from_utf8(output.stdout).ok()?;
            let json: Value = serde_json::from_str(&json_string).ok()?;
            Some(PathBuf::from(json.get("target_directory")?.as_str()?))
        });

        let mut path = target_dir.unwrap_or(self.project_out_directory.join("target"));

        if let Some(triple) = self.target_triple.as_ref() {
            path.push(triple);
        }
        path.push(if self.profile == "dev" {
            "debug"
        } else {
            self.profile.as_str()
        });
        if let &BuildArtifact::Example(_) = build_artifact {
            path.push("examples");
        }
        path
    }

    pub fn get_project_dir(&self) -> &Path {
        &self.project_out_directory.as_path()
    }

    pub fn get_bundle_settings(&self, build_artifact: &BuildArtifact) -> (BundleSettings, String) {
        let bundle_settings =
            bundle_settings_of_package(&self.package, &self.package_type).expect("");
        let bundle_settings = bundle_settings_with_artifact(bundle_settings, build_artifact);
        match &build_artifact {
            BuildArtifact::Main => {
                if let Some(target) = self
                    .package
                    .targets
                    .iter()
                    .find(|target| target.kind.contains(&TargetKind::Bin))
                {
                    (bundle_settings, target.name.clone())
                } else {
                    panic!(
                        "No `bin` target is found in package '{}'",
                        self.package.name
                    );
                }
            }
            BuildArtifact::Bin(name) => (bundle_settings, name.clone()),
            BuildArtifact::Example(name) => (bundle_settings, name.clone()),
        }
    }
}

fn bundle_settings_with_artifact(
    opt_map: BundleSettings,
    artifact: &BuildArtifact,
) -> BundleSettings {
    match artifact {
        BuildArtifact::Main => opt_map,
        BuildArtifact::Bin(name) => {
            if let Some(extra_bundle_settings) = opt_map.bin.get(name) {
                extra_bundle_settings.clone().merge(opt_map)
            } else {
                _ = print_warning(&format!(
                    "No [package.metadata.bundle.bin.{name}] section in Cargo.toml"
                ));
                opt_map
            }
        }
        BuildArtifact::Example(example_name) => {
            if let Some(extra_bundle_settings) = opt_map.bin.get(example_name) {
                extra_bundle_settings.clone().merge(opt_map)
            } else {
                _ = print_warning(&format!(
                    "No [package.metadata.bundle.example.{example_name}] section in Cargo.toml"
                ));
                opt_map
            }
        }
    }
}

/// Try to load `Cargo.toml` file in the specified directory
fn load_metadata(dir: &Path) -> crate::Result<Metadata> {
    let cargo_file_path = dir.join("Cargo.toml");
    Ok(MetadataCommand::new()
        .manifest_path(cargo_file_path)
        .exec()?)
}

/*
    The specification of the Cargo.toml Manifest that covers the "workspace" section is here:
    https://doc.rust-lang.org/cargo/reference/manifest.html#the-workspace-section

    Determining if the current project folder is part of a workspace:
        - Walk up the file system, looking for a Cargo.toml file.
        - Stop at the first one found.
        - If one is found before reaching "/" then this folder belongs to that parent workspace
*/
fn get_workspace_dir(current_dir: PathBuf) -> PathBuf {
    let mut dir = current_dir.clone();
    let set = load_metadata(&dir);
    if set.is_ok() {
        return dir;
    }
    while dir.pop() {
        let set = load_metadata(&dir);
        if set.is_ok() {
            return dir;
        }
    }

    // Nothing found walking up the file system, return the starting directory
    current_dir
}

fn bundle_settings_of_package(
    package: &Package,
    format: &PackageType,
) -> crate::Result<BundleSettings> {
    if let Some(bundle) = package.metadata.get("bundle") {
        let settings = serde_json::from_value::<BundleSettings>(bundle.clone())?;
        if let Some(extra) = settings.targets.get(format.short_name()) {
            return Ok(extra.clone().merge(settings));
        }
        return Ok(settings);
    }
    print_warning(&format!(
        "No [package.metadata.bundle] section in package \"{}\"",
        package.name
    ))?;
    Ok(BundleSettings::default())
}

impl TryFrom<(&Cli, PackageType)> for BundleTargetInfo {
    fn try_from(value: (&Cli, PackageType)) -> Result<Self, String> {
        let target = value.0.get_target();
        let profile = if value.0.release {
            "release".to_string()
        } else if let Some(profile) = value.0.profile.as_ref() {
            if profile == "debug" {
                return Err("Profile name `debug` is reserved".to_string());
            }
            profile.to_string()
        } else {
            "dev".to_string()
        };
        let package_name = value.0.package.as_deref().map(|s| s.to_string());
        let workspace_dir = get_workspace_dir(value.0.dir.clone());
        let cargo_settings = load_metadata(&workspace_dir).expect("1");
        let package = match &package_name {
            Some(package) => cargo_settings
                .packages
                .iter()
                .find(|p| p.name.as_str() == package)
                .ok_or_else(|| anyhow::anyhow!("Package '{package}' not found in workspace")),
            None => cargo_settings
                .root_package()
                .ok_or_else(|| anyhow::anyhow!("No root package found in workspace")),
        }
        .expect("msg");

        let (target_triple, target_info) = match target {
            Some((triple, target_info)) => (Some(triple), target_info),
            None => (None, None),
        };
        Ok(Self {
            target_info,
            target_triple,
            package_type: value.1,
            project_out_directory: workspace_dir,
            profile,
            package: package.to_owned(),
        })
    }

    type Error = String;
}

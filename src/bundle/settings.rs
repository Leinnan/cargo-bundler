use crate::bundle::target_info::BundleTargetInfo;

use super::category::AppCategory;
use super::common::print_warning;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Display;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageType {
    OsxBundle,
    IosBundle,
    WindowsMsi,
    WxsMsi,
    Deb,
    Rpm,
    AppImage,
}

impl PackageType {
    pub fn bundle_project(&self, settings: &Settings) -> crate::Result<Vec<PathBuf>> {
        match self {
            PackageType::OsxBundle => super::osx_bundle::bundle_project(&settings),
            PackageType::IosBundle => super::ios_bundle::bundle_project(&settings),
            PackageType::WindowsMsi => super::msi_bundle::bundle_project(&settings),
            PackageType::WxsMsi => super::wxsmsi_bundle::bundle_project(&settings),
            PackageType::Deb => super::linux::deb_bundle::bundle_project(&settings),
            PackageType::Rpm => super::linux::rpm_bundle::bundle_project(&settings),
            PackageType::AppImage => super::linux::appimage_bundle::bundle_project(&settings),
        }
    }
}

impl std::str::FromStr for PackageType {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        PackageType::try_from(s)
    }
}

impl std::fmt::Display for PackageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.short_name())
    }
}

impl TryFrom<&str> for PackageType {
    type Error = anyhow::Error;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        PackageType::from_short_name(s).ok_or_else(|| {
            let all = PackageType::all()
                .iter()
                .map(|&s| s.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            anyhow::anyhow!("Unsupported package type: '{s}'. Supported types are: {all}")
        })
    }
}

impl TryFrom<String> for PackageType {
    type Error = anyhow::Error;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        PackageType::try_from(s.as_str())
    }
}

impl PackageType {
    pub fn from_short_name(name: &str) -> Option<PackageType> {
        // Other types we may eventually want to support: apk
        match name {
            "deb" => Some(PackageType::Deb),
            "ios" => Some(PackageType::IosBundle),
            "msi" => Some(PackageType::WindowsMsi),
            "wxsmsi" => Some(PackageType::WxsMsi),
            "osx" => Some(PackageType::OsxBundle),
            "rpm" => Some(PackageType::Rpm),
            "appimage" => Some(PackageType::AppImage),
            _ => None,
        }
    }

    pub const fn short_name(&self) -> &'static str {
        match *self {
            PackageType::Deb => "deb",
            PackageType::IosBundle => "ios",
            PackageType::WindowsMsi => "msi",
            PackageType::WxsMsi => "wxsmsi",
            PackageType::OsxBundle => "osx",
            PackageType::Rpm => "rpm",
            PackageType::AppImage => "appimage",
        }
    }

    pub const fn all() -> &'static [&'static str] {
        &["deb", "ios", "msi", "wxsmsi", "osx", "rpm", "appimage"]
    }
}

#[derive(Clone, Debug)]
pub enum BuildArtifact {
    Main,
    Bin(String),
    Example(String),
}

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct BundleSettings {
    // General settings:
    name: Option<String>,
    identifier: Option<String>,
    icon: Option<Vec<String>>,
    version: Option<String>,
    resources: Option<Vec<String>>,
    copyright: Option<String>,
    category: Option<AppCategory>,
    short_description: Option<String>,
    long_description: Option<String>,
    // OS-specific settings:
    linux_mime_types: Option<Vec<String>>,
    linux_exec_args: Option<String>,
    linux_use_terminal: Option<bool>,
    deb_depends: Option<Vec<String>>,
    osx_frameworks: Option<Vec<String>>,
    osx_plugins: Option<Vec<String>>,
    osx_minimum_system_version: Option<String>,
    osx_url_schemes: Option<Vec<String>>,
    osx_info_plist_exts: Option<Vec<String>>,
    // Bundles for other binaries/examples:
    pub bin: Option<HashMap<String, BundleSettings>>,
    pub example: Option<HashMap<String, BundleSettings>>,
}

#[derive(Clone, Debug)]
pub struct Settings {
    pub target: BundleTargetInfo,
    features: Option<String>,
    build_artifact: BuildArtifact,
    all_features: bool,
    no_default_features: bool,
    bundle_settings: BundleSettings,
}

impl Settings {
    pub fn get_target_dir(&self) -> PathBuf {
        self.target.get_target_dir(&self.build_artifact)
    }

    pub fn new(bundle_info: &BundleTargetInfo, cli: &crate::Cli) -> crate::Result<Self> {
        let build_artifact = if let Some(bin) = cli.bin.as_ref() {
            BuildArtifact::Bin(bin.to_string())
        } else if let Some(example) = cli.example.as_ref() {
            BuildArtifact::Example(example.to_string())
        } else {
            BuildArtifact::Main
        };
        let all_features = cli.all_features;
        let no_default_features = cli.no_default_features;
        let features = cli.features.as_ref().map(|features| features.into());
        let bundle_settings = bundle_info.get_bundle_settings(&build_artifact).0;

        Ok(Settings {
            target: bundle_info.clone(),
            features,
            build_artifact,
            all_features,
            no_default_features,
            bundle_settings,
        })
    }

    /// Returns the architecture for the binary being bundled (e.g. "arm" or
    /// "x86" or "x86_64").
    pub fn binary_arch(&self) -> &str {
        if let Some(ref info) = self.target.target_info {
            info.target_arch()
        } else {
            std::env::consts::ARCH
        }
    }

    /// Returns the file name of the binary being bundled.
    pub fn binary_name(&self) -> String {
        self.target
            .get_bundle_settings(&self.build_artifact)
            .1
            .clone()
    }

    /// Returns the path to the binary being bundled.
    pub fn binary_path(&self, target: PackageType) -> PathBuf {
        let binary_name = self.binary_name();
        match target {
            PackageType::WindowsMsi | PackageType::WxsMsi => {
                self.get_target_dir().join(format!("{}.exe", binary_name))
            }
            _ => self.get_target_dir().join(&binary_name),
        }
    }

    /// If the bundle is being cross-compiled, returns the target triple string
    /// (e.g. `"x86_64-apple-darwin"`).  If the bundle is targeting the host
    /// environment, returns `None`.
    pub fn target_triple(&self) -> Option<&str> {
        match self.target.target_triple {
            Some(ref triple) => Some(triple.as_str()),
            None => None,
        }
    }

    pub fn features(&self) -> Option<&str> {
        match self.features {
            Some(ref features) => Some(features.as_str()),
            None => None,
        }
    }

    /// Returns the artifact that is being bundled.
    pub fn build_artifact(&self) -> &BuildArtifact {
        &self.build_artifact
    }

    /// Returns `release`, 'dev` or other profile.
    pub fn build_profile(&self) -> &str {
        &self.target.profile
    }

    pub fn all_features(&self) -> bool {
        self.all_features
    }

    pub fn no_default_features(&self) -> bool {
        self.no_default_features
    }

    pub fn bundle_name(&self) -> String {
        self.target
            .get_bundle_settings(&self.build_artifact)
            .0
            .name
            .unwrap_or(self.target.package.name.to_string())
    }

    pub fn bundle_identifier(&self) -> Cow<'_, str> {
        if let Some(identifier) = &self.bundle_settings.identifier {
            identifier.into()
        } else {
            match &self.build_artifact {
                BuildArtifact::Main => "".into(),
                BuildArtifact::Bin(name) => format!("{name}.{}", self.target.package.name).into(),
                BuildArtifact::Example(name) => {
                    format!("{name}.example.{}", self.target.package.name).into()
                }
            }
        }
    }

    /// Returns an iterator over the icon files to be used for this bundle.
    pub fn icon_files(&self) -> ResourcePaths<'_> {
        match self.bundle_settings.icon {
            Some(ref paths) => ResourcePaths::new(paths.as_slice(), false),
            None => ResourcePaths::new(&[], false),
        }
    }

    /// Returns an iterator over the resource files to be included in this
    /// bundle.
    pub fn resource_files(&self) -> ResourcePaths<'_> {
        match self.bundle_settings.resources {
            Some(ref paths) => ResourcePaths::new(paths.as_slice(), true),
            None => ResourcePaths::new(&[], true),
        }
    }

    pub fn version_string(&self) -> &dyn Display {
        match self.bundle_settings.version.as_ref() {
            Some(v) => v,
            None => &self.target.package.version,
        }
    }

    pub fn copyright_string(&self) -> Option<&str> {
        self.bundle_settings.copyright.as_deref()
    }

    pub fn author_names(&self) -> &[String] {
        &self.target.package.authors
    }

    pub fn authors_comma_separated(&self) -> Option<String> {
        let names = self.author_names();
        if names.is_empty() {
            None
        } else {
            Some(names.join(", "))
        }
    }

    pub fn homepage_url(&self) -> &str {
        self.target.package.homepage.as_deref().unwrap_or("")
    }

    pub fn app_category(&self) -> Option<AppCategory> {
        self.bundle_settings.category
    }

    pub fn short_description(&self) -> &str {
        self.bundle_settings
            .short_description
            .as_deref()
            .unwrap_or_else(|| self.target.package.description.as_deref().unwrap_or(""))
    }

    pub fn long_description(&self) -> Option<&str> {
        self.bundle_settings.long_description.as_deref()
    }

    pub fn license_content(&self) -> Option<String> {
        self.target
            .package
            .license_file
            .as_ref()
            .and_then(|license_file| {
                let license_path = self.target.get_project_dir().join(license_file);
                match std::fs::read_to_string(&license_path) {
                    Ok(content) => Some(content),
                    Err(err) => {
                        print_warning(&format!(
                            "Failed to read license file '{license_path:?}': {err} -- ignoring",
                        ))
                        .ok();
                        None
                    }
                }
            })
            .or_else(|| self.target.package.license.as_ref().map(|s| s.to_string()))
    }

    pub fn debian_dependencies(&self) -> &[String] {
        match self.bundle_settings.deb_depends {
            Some(ref dependencies) => dependencies.as_slice(),
            None => &[],
        }
    }

    pub fn linux_mime_types(&self) -> &[String] {
        match self.bundle_settings.linux_mime_types {
            Some(ref mime_types) => mime_types.as_slice(),
            None => &[],
        }
    }

    pub fn linux_use_terminal(&self) -> Option<bool> {
        self.bundle_settings.linux_use_terminal
    }

    pub fn linux_exec_args(&self) -> Option<&str> {
        self.bundle_settings.linux_exec_args.as_deref()
    }

    pub fn osx_frameworks(&self) -> &[String] {
        match self.bundle_settings.osx_frameworks {
            Some(ref frameworks) => frameworks.as_slice(),
            None => &[],
        }
    }

    pub fn osx_plugins(&self) -> &[String] {
        match self.bundle_settings.osx_plugins {
            Some(ref plugins) => plugins.as_slice(),
            None => &[],
        }
    }

    pub fn osx_minimum_system_version(&self) -> Option<&str> {
        self.bundle_settings.osx_minimum_system_version.as_deref()
    }

    pub fn osx_url_schemes(&self) -> &[String] {
        match self.bundle_settings.osx_url_schemes {
            Some(ref urlosx_url_schemes) => urlosx_url_schemes.as_slice(),
            None => &[],
        }
    }

    /// Returns an iterator over the plist files for this bundle
    pub fn osx_info_plist_exts(&self) -> ResourcePaths<'_> {
        match self.bundle_settings.osx_info_plist_exts {
            Some(ref paths) => ResourcePaths::new(paths.as_slice(), false),
            None => ResourcePaths::new(&[], false),
        }
    }
}

pub struct ResourcePaths<'a> {
    pattern_iter: std::slice::Iter<'a, String>,
    glob_iter: Option<glob::Paths>,
    walk_iter: Option<walkdir::IntoIter>,
    allow_walk: bool,
}

impl<'a> ResourcePaths<'a> {
    fn new(patterns: &'a [String], allow_walk: bool) -> ResourcePaths<'a> {
        ResourcePaths {
            pattern_iter: patterns.iter(),
            glob_iter: None,
            walk_iter: None,
            allow_walk,
        }
    }
}

impl Iterator for ResourcePaths<'_> {
    type Item = crate::Result<PathBuf>;

    fn next(&mut self) -> Option<crate::Result<PathBuf>> {
        loop {
            if let Some(ref mut walk_entries) = self.walk_iter
                && let Some(entry) = walk_entries.next()
            {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(error) => return Some(Err(anyhow::Error::from(error))),
                };
                let path = entry.path();
                if path.is_dir() {
                    continue;
                }
                return Some(Ok(path.to_path_buf()));
            }
            self.walk_iter = None;
            if let Some(ref mut glob_paths) = self.glob_iter
                && let Some(glob_result) = glob_paths.next()
            {
                let path = match glob_result {
                    Ok(path) => path,
                    Err(error) => return Some(Err(anyhow::Error::from(error))),
                };
                if path.is_dir() {
                    if self.allow_walk {
                        let walk = walkdir::WalkDir::new(path);
                        self.walk_iter = Some(walk.into_iter());
                        continue;
                    } else {
                        return Some(Err(anyhow::anyhow!("{path:?} is a directory")));
                    }
                }
                return Some(Ok(path));
            }
            self.glob_iter = None;
            if let Some(pattern) = self.pattern_iter.next() {
                let glob = match glob::glob(pattern) {
                    Ok(glob) => glob,
                    Err(error) => return Some(Err(anyhow::Error::from(error))),
                };
                self.glob_iter = Some(glob);
                continue;
            }
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AppCategory, BundleSettings};

    #[test]
    fn parse_cargo_toml() {
        let toml_str = "\
            name = \"Example Application\"\n\
            identifier = \"com.example.app\"\n\
            resources = [\"data\", \"foo/bar\"]\n\
            category = \"Puzzle Game\"\n\
            long_description = \"\"\"\n\
            This is an example of a\n\
            simple application.\n\
            \"\"\"\n";
        let bundle: BundleSettings = toml::from_str(toml_str).unwrap();
        assert_eq!(bundle.name, Some("Example Application".to_string()));
        assert_eq!(bundle.identifier, Some("com.example.app".to_string()));
        assert_eq!(bundle.icon, None);
        assert_eq!(bundle.version, None);
        assert_eq!(
            bundle.resources,
            Some(vec!["data".to_string(), "foo/bar".to_string()])
        );
        assert_eq!(bundle.category, Some(AppCategory::PuzzleGame));
        assert_eq!(
            bundle.long_description,
            Some(
                "This is an example of a\n\
                         simple application.\n"
                    .to_string()
            )
        );
    }

    #[test]
    fn parse_bin_and_example_bundles() {
        let toml_str = "\
            [bin.foo]\n\
            name = \"Foo App\"\n\
            \n\
            [bin.bar]\n\
            name = \"Bar App\"\n\
            \n\
            [example.baz]\n\
            name = \"Baz Example\"\n";
        let bundle: BundleSettings = toml::from_str(toml_str).unwrap();
        assert!(bundle.example.is_some());

        let bins = bundle.bin.as_ref().unwrap();
        assert!(bins.contains_key("foo"));
        let foo: &BundleSettings = bins.get("foo").unwrap();
        assert_eq!(foo.name, Some("Foo App".to_string()));
        assert!(bins.contains_key("bar"));
        let bar: &BundleSettings = bins.get("bar").unwrap();
        assert_eq!(bar.name, Some("Bar App".to_string()));

        let examples = bundle.example.as_ref().unwrap();
        assert!(examples.contains_key("baz"));
        let baz: &BundleSettings = examples.get("baz").unwrap();
        assert_eq!(baz.name, Some("Baz Example".to_string()));
    }
}

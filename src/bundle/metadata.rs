use std::collections::HashMap;

use crate::bundle::category::AppCategory;

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct BundleSettings {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    pub identifier: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub icon: Vec<String>,
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resources_mapping: Vec<(String, String)>,
    pub copyright: Option<String>,
    pub category: Option<AppCategory>,
    pub short_description: Option<String>,
    pub long_description: Option<String>,
    // OS-specific settings:
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub linux_mime_types: Vec<String>,
    pub linux_exec_args: Option<String>,
    pub linux_use_terminal: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deb_depends: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub osx_frameworks: Vec<String>,
    pub osx_plugins: Option<Vec<String>>,
    pub osx_minimum_system_version: Option<String>,
    pub osx_url_schemes: Option<Vec<String>>,
    pub osx_info_plist_exts: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub targets: HashMap<String, BundleSettings>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub bin: HashMap<String, BundleSettings>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub example: HashMap<String, BundleSettings>,
}

impl BundleSettings {
    pub fn merge(self, other: BundleSettings) -> Self {
        Self {
            name: if self.name.is_empty() {
                other.name
            } else {
                self.name
            },
            targets: self.targets.into_iter().chain(other.targets).collect(),
            identifier: self.identifier.or(other.identifier),
            icon: if self.icon.is_empty() {
                self.icon
            } else {
                other.icon
            },
            version: self.version.or(other.version),
            resources_mapping: if self.resources_mapping.is_empty() {
                other.resources_mapping
            } else {
                self.resources_mapping
            },
            copyright: self.copyright.or(other.copyright),
            category: self.category.or(other.category),
            short_description: self.short_description.or(other.short_description),
            long_description: self.long_description.or(other.long_description),
            linux_mime_types: if self.linux_mime_types.is_empty() {
                other.linux_mime_types
            } else {
                self.linux_mime_types
            },
            linux_exec_args: self.linux_exec_args.or(other.linux_exec_args),
            linux_use_terminal: self.linux_use_terminal.or(other.linux_use_terminal),
            deb_depends: if self.deb_depends.is_empty() {
                other.deb_depends
            } else {
                self.deb_depends
            },
            osx_frameworks: if self.osx_frameworks.is_empty() {
                other.osx_frameworks
            } else {
                self.osx_frameworks
            },
            osx_plugins: self.osx_plugins.or(other.osx_plugins),
            osx_minimum_system_version: self
                .osx_minimum_system_version
                .or(other.osx_minimum_system_version),
            osx_url_schemes: self.osx_url_schemes.or(other.osx_url_schemes),
            osx_info_plist_exts: self.osx_info_plist_exts.or(other.osx_info_plist_exts),
            bin: self.bin.into_iter().chain(other.bin).collect(),
            example: self.example.into_iter().chain(other.example).collect(),
        }
    }
}

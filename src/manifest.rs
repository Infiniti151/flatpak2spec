// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Infiniti151

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct FlatpakManifest {
    pub id: Option<String>,
    #[serde(rename = "app-id")]
    pub app_id: Option<String>,
    pub runtime: Option<String>,
    pub runtime_version: Option<String>,
    pub sdk: Option<String>,
    pub command: Option<String>,
    pub buildsystem: Option<String>,
    pub modules: Option<Vec<Module>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Module {
    Path(String),
    Detail(ModuleDetail),
}

impl Default for Module {
    fn default() -> Self {
        Module::Path(String::new())
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct ModuleDetail {
    pub name: String,
    pub buildsystem: Option<String>,
    pub sources: Option<Vec<Source>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Source {
    /// External source file reference (e.g. "cargo-sources.json")
    Path(String),
    /// Structured source object (e.g. {"type": "dir", "path": "."})
    Detail(SourceDetail),
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum SourceDetail {
    #[serde(rename = "archive")]
    Archive { url: String, sha256: Option<String> },
    #[serde(rename = "git")]
    Git { url: String, tag: Option<String> },
    #[serde(rename = "dir")]
    Dir { path: String },
    #[default]
    #[serde(other)]
    Other,
}

impl FlatpakManifest {
    pub fn load_from_workspace(workspace_path: &Path) -> Result<Self> {
        let manifest_path = find_manifest_file(workspace_path)?;
        let content = fs::read_to_string(&manifest_path).with_context(|| {
            format!("Failed to read manifest file: {}", manifest_path.display())
        })?;

        let manifest: FlatpakManifest =
            if manifest_path.extension().and_then(|s| s.to_str()) == Some("json") {
                serde_json::from_str(&content).with_context(|| {
                    format!("Failed to parse JSON manifest: {}", manifest_path.display())
                })?
            } else {
                serde_yml::from_str(&content).with_context(|| {
                    format!("Failed to parse YAML manifest: {}", manifest_path.display())
                })?
            };

        manifest.validate_meson(workspace_path)?;

        Ok(manifest)
    }

    pub fn get_app_id(&self) -> Option<&str> {
        self.id.as_deref().or(self.app_id.as_deref())
    }

    /// Access basic metadata details to ensure fields are inspected
    pub fn get_target_info(&self) -> (Option<&str>, Option<&str>, Option<&str>, Option<&str>) {
        (
            self.runtime.as_deref(),
            self.runtime_version.as_deref(),
            self.sdk.as_deref(),
            self.command.as_deref(),
        )
    }

    pub fn validate_meson(&self, workspace_path: &Path) -> Result<()> {
        if let Some(ref bs) = self.buildsystem {
            if bs.to_lowercase() != "meson" {
                bail!(
                    "Unsupported buildsystem '{}'. flatpak2spec currently only supports Meson-based applications.",
                    bs
                );
            }
        }

        if let Some(ref modules) = self.modules {
            let total_modules = modules.len();
            for module in modules {
                if let Module::Detail(detail) = module {
                    if is_main_app_module(detail, self.get_app_id(), total_modules) {
                        let bs = detail.buildsystem.as_deref().unwrap_or("meson");
                        if bs.to_lowercase() != "meson" {
                            bail!(
                                "Main application module '{}' uses buildsystem '{}'. flatpak2spec currently only supports 'meson'.",
                                detail.name,
                                bs
                            );
                        }
                    }
                }
            }
        }

        let meson_build = workspace_path.join("meson.build");
        if !meson_build.exists() {
            bail!(
                "Missing 'meson.build' file in workspace root ({}). This repository is not a Meson project.",
                workspace_path.display()
            );
        }

        Ok(())
    }
}

fn find_manifest_file(workspace_path: &Path) -> Result<std::path::PathBuf> {
    let entries = fs::read_dir(workspace_path)
        .with_context(|| format!("Failed to read directory: {}", workspace_path.display()))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if matches!(ext, "json" | "yaml" | "yml") {
                    let filename = path.file_name().unwrap_or_default().to_string_lossy();

                    // Skip hidden files
                    if filename.starts_with('.') {
                        continue;
                    }

                    // Flatpak manifest pattern: Reverse-DNS (e.g. org.gnome.App.json) or ends with 'manifest'
                    if filename.matches('.').count() >= 2
                        || filename.to_lowercase().contains("manifest")
                    {
                        return Ok(path);
                    }
                }
            }
        }
    }

    bail!(
        "No Flatpak manifest (.json, .yaml, .yml) found in {}",
        workspace_path.display()
    )
}

fn is_main_app_module(module: &ModuleDetail, app_id: Option<&str>, total_modules: usize) -> bool {
    // 1. If it's the only module in the manifest, it's definitely the main app
    if total_modules == 1 {
        return true;
    }

    let id_lower = app_id.map(|s| s.to_lowercase());
    let name_lower = module.name.to_lowercase();

    // 2. Exact match against App ID or its last reverse-DNS segment (e.g. "gitte" == "gitte")
    if let Some(ref id) = id_lower {
        let last_segment = id.split('.').last().unwrap_or(id);
        if name_lower == *id || name_lower == last_segment {
            return true;
        }
    }

    // 3. Check sources for local directory root "."
    if let Some(ref sources) = module.sources {
        for source in sources {
            if let Source::Detail(detail) = source {
                match detail {
                    SourceDetail::Dir { path } => {
                        if path == "." || path == "./" {
                            return true;
                        }
                    }
                    SourceDetail::Archive { url, .. } | SourceDetail::Git { url, .. } => {
                        if let Some(ref id) = id_lower {
                            let last_segment = id.split('.').last().unwrap_or(id);
                            let url_lower = url.to_lowercase();
                            if url_lower.contains(id) || url_lower.contains(last_segment) {
                                return true;
                            }
                        }
                    }
                    SourceDetail::Other => {}
                }
            }
        }
    }

    false
}

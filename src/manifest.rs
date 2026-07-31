// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Infiniti151

use crate::utils;
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct FlatpakManifest {
    // Accepts "id", "app-id", or "app_id" seamlessly
    #[serde(alias = "app-id", alias = "app_id")]
    pub id: Option<String>,

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
    pub modules: Option<Vec<Module>>,
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
        // Try locating an actual manifest file first
        let manifest = match find_manifest_file(workspace_path) {
            Ok(manifest_path) => {
                let content = fs::read_to_string(&manifest_path).with_context(|| {
                    format!("Failed to read manifest file: {}", manifest_path.display())
                })?;

                if manifest_path.extension().and_then(|s| s.to_str()) == Some("json") {
                    serde_json::from_str(&content).with_context(|| {
                        format!("Failed to parse JSON manifest: {}", manifest_path.display())
                    })?
                } else {
                    serde_norway::from_str(&content).with_context(|| {
                        format!("Failed to parse YAML manifest: {}", manifest_path.display())
                    })?
                }
            }
            Err(_) => {
                // Fallback: No manifest file was found, synthesize a default manifest from repo files
                let mut synthetic = FlatpakManifest::default();
                synthetic.id = synthetic.resolve_app_id(workspace_path);
                // Force buildsystem to meson since meson.build presence is validated right after
                synthetic.buildsystem = Some("meson".to_string());

                // If snapcraft.yaml exists, try extracting command/name from it
                if let Ok(content) = fs::read_to_string(workspace_path.join("snap/snapcraft.yaml"))
                {
                    if let Ok(val) = serde_norway::from_str::<serde_json::Value>(&content) {
                        if synthetic.command.is_none() {
                            synthetic.command =
                                val.get("name").and_then(|v| v.as_str()).map(String::from);
                        }
                    }
                }

                synthetic
            }
        };

        manifest.validate_meson(workspace_path)?;

        Ok(manifest)
    }

    /// Takes 0 arguments for backward compatibility. Returns the raw manifest App ID if set.
    pub fn get_app_id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// Resolves the App ID by prioritizing:
    /// 1. Direct manifest `id` / `app-id` field
    /// 2. Reverse-DNS AppStream metadata files (`*.metainfo.xml`, `*.appdata.xml`)
    /// 3. `app_id` / `application_id` declarations inside `meson.build`
    pub fn resolve_app_id(&self, workspace_path: &Path) -> Option<String> {
        if let Some(id) = self.get_app_id() {
            if !id.is_empty() {
                return Some(id.to_string());
            }
        }

        if let Some(app_id) = find_app_id_from_metainfo(workspace_path) {
            return Some(app_id);
        }

        if let Some(app_id) = find_app_id_from_meson(workspace_path) {
            return Some(app_id);
        }

        None
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
        let meson_build = workspace_path.join("meson.build");
        if !meson_build.exists() {
            bail!(
                "Missing 'meson.build' file in workspace root ({}). This repository is not a Meson project.",
                workspace_path.display()
            );
        }

        // 1. If manifest top-level buildsystem is meson, or it was synthesized as meson
        if let Some(ref bs) = self.buildsystem {
            if bs.to_lowercase() == "meson" {
                return Ok(());
            }
        }

        // 2. Search modules array if present
        if let Some(ref modules) = self.modules {
            let resolved_id = self.resolve_app_id(workspace_path);
            if has_meson_app_module(modules, resolved_id.as_deref()) {
                return Ok(());
            }
        } else {
            // 3. If modules is None/empty, presence of root meson.build is sufficient
            return Ok(());
        }

        bail!(
            "Could not find a Meson-based main application module in the Flatpak manifest. flatpak2spec currently requires a Meson build system."
        );
    }
}

fn find_manifest_file(workspace_path: &Path) -> Result<std::path::PathBuf> {
    // 1. First search workspace root
    let candidates = utils::find_matching_files(workspace_path, &[".json", ".yaml", ".yml"]);

    for path in candidates {
        if is_likely_manifest(&path) {
            return Ok(path);
        }
    }

    // 2. Search common packaging subdirectories
    let subdirs = [
        "build-aux",
        "flatpak",
        "packaging",
        ".flatpak",
        "dist",
        "build-aux/flatpak",
    ];
    for subdir in subdirs {
        let dir = workspace_path.join(subdir);
        if dir.is_dir() {
            let sub_candidates = utils::find_matching_files(&dir, &[".json", ".yaml", ".yml"]);
            for path in sub_candidates {
                if is_likely_manifest(&path) {
                    return Ok(path);
                }
            }
        }
    }

    bail!(
        "No Flatpak manifest (.json, .yaml, .yml) found in {} or common subdirectories (build-aux, flatpak, packaging).",
        workspace_path.display()
    )
}

fn is_likely_manifest(path: &Path) -> bool {
    if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
        let name_lower = filename.to_lowercase();

        // Match reverse-DNS naming (e.g., io.github.nacho.mundi.json) or files containing "manifest"
        if filename.matches('.').count() >= 2 || name_lower.contains("manifest") {
            // Ignore common non-manifest YAML/JSON files like snapcraft, CI configs, or translation configs
            if name_lower.contains("snapcraft")
                || name_lower.contains("ci")
                || name_lower.starts_with('.')
            {
                return false;
            }
            return true;
        }
    }
    false
}

fn has_meson_app_module(modules: &[Module], app_id: Option<&str>) -> bool {
    let total_modules = modules.len();

    for module in modules {
        if let Module::Detail(detail) = module {
            let bs = detail.buildsystem.as_deref().unwrap_or("meson");

            // If this module is Meson-based AND identified as the main application module
            if bs.to_lowercase() == "meson" && is_main_app_module(detail, app_id, total_modules) {
                return true;
            }

            // Recursively search child modules (e.g. sub-modules array)
            if let Some(ref child_modules) = detail.modules {
                if has_meson_app_module(child_modules, app_id) {
                    return true;
                }
            }
        }
    }

    false
}

fn is_main_app_module(module: &ModuleDetail, app_id: Option<&str>, total_modules: usize) -> bool {
    let id_lower = app_id.map(|s| s.to_lowercase());
    let name_lower = module.name.to_lowercase();

    // 1. Exact or partial match against App ID (e.g., "missioncenter" matches "io.missioncenter.MissionCenter")
    if let Some(ref id) = id_lower {
        let last_segment = id.split('.').last().unwrap_or(id).to_lowercase();
        if name_lower == *id || name_lower == last_segment {
            return true;
        }
    }

    // 2. Check sources for local directory root ("." or "..")
    if let Some(ref sources) = module.sources {
        for source in sources {
            if let Source::Detail(detail) = source {
                match detail {
                    SourceDetail::Dir { path } => {
                        if path == "." || path == "./" || path == ".." || path == "../" {
                            return true;
                        }
                    }
                    SourceDetail::Archive { url, .. } | SourceDetail::Git { url, .. } => {
                        if let Some(ref id) = id_lower {
                            let last_segment = id.split('.').last().unwrap_or(id).to_lowercase();
                            let url_lower = url.to_lowercase();
                            if url_lower.contains(id) || url_lower.contains(&last_segment) {
                                return true;
                            }
                        }
                    }
                    SourceDetail::Other => {}
                }
            }
        }
    }

    // 3. Fallback: single module manifests default to main app
    if total_modules == 1 {
        return true;
    }

    false
}

fn find_app_id_from_metainfo(workspace_path: &Path) -> Option<String> {
    let matches = utils::find_matching_files(workspace_path, &[".metainfo.xml", ".appdata.xml"]);

    for path in matches {
        if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
            let clean_name = file_name
                .trim_end_matches(".xml")
                .trim_end_matches(".metainfo")
                .trim_end_matches(".appdata");

            if clean_name.matches('.').count() >= 2 {
                return Some(clean_name.to_string());
            }
        }
    }
    None
}

fn find_app_id_from_meson(workspace_path: &Path) -> Option<String> {
    let meson_files = vec![
        workspace_path.join("meson.build"),
        workspace_path.join("data").join("meson.build"),
    ];

    for meson_path in meson_files {
        if let Ok(content) = fs::read_to_string(meson_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("app_id") || trimmed.starts_with("application_id") {
                    if let Some(val) = trimmed.split('=').nth(1) {
                        let clean = val.trim().trim_matches('\'').trim_matches('"');
                        if clean.matches('.').count() >= 2 {
                            return Some(clean.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

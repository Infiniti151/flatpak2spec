// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Infiniti151

use crate::utils;
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct FlatpakManifest {
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
    Path(String),
    Detail(SourceDetail),
}

impl Module {
    /// Resolves external file paths or returns the inline module details.
    pub fn resolve(&self, base_dir: &Path) -> Result<ModuleDetail> {
        match self {
            Module::Detail(detail) => Ok(detail.clone()),
            Module::Path(path_str) => {
                let file_path = base_dir.join(path_str);
                let content = fs::read_to_string(&file_path).with_context(|| {
                    format!(
                        "Failed to read external module file: {}",
                        file_path.display()
                    )
                })?;

                let is_json = file_path.extension().and_then(|s| s.to_str()) == Some("json");
                let detail: ModuleDetail = if is_json {
                    serde_json::from_str(&content).with_context(|| {
                        format!("Failed to parse JSON module file: {}", file_path.display())
                    })?
                } else {
                    serde_norway::from_str(&content).with_context(|| {
                        format!("Failed to parse YAML module file: {}", file_path.display())
                    })?
                };

                Ok(detail)
            }
        }
    }
}

impl Source {
    /// Resolves external file paths or returns the inline source details.
    pub fn resolve(&self, base_dir: &Path) -> Result<SourceDetail> {
        match self {
            Source::Detail(detail) => Ok(detail.clone()),
            Source::Path(path_str) => {
                let file_path = base_dir.join(path_str);
                let content = fs::read_to_string(&file_path).with_context(|| {
                    format!(
                        "Failed to read external source file: {}",
                        file_path.display()
                    )
                })?;

                let is_json = file_path.extension().and_then(|s| s.to_str()) == Some("json");
                let detail: SourceDetail = if is_json {
                    serde_json::from_str(&content).with_context(|| {
                        format!("Failed to parse JSON source file: {}", file_path.display())
                    })?
                } else {
                    serde_norway::from_str(&content).with_context(|| {
                        format!("Failed to parse YAML source file: {}", file_path.display())
                    })?
                };

                Ok(detail)
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum SourceDetail {
    #[serde(rename = "archive")]
    Archive { url: String },
    #[serde(rename = "git")]
    Git { url: String },
    #[serde(rename = "dir")]
    Dir { path: String },
    #[default]
    #[serde(other)]
    Other,
}

impl FlatpakManifest {
    pub fn load_from_workspace(workspace_path: &Path) -> Result<Self> {
        let mut manifest = match find_manifest_file(workspace_path) {
            Ok(manifest_path) => {
                let content = fs::read_to_string(&manifest_path).with_context(|| {
                    format!("Failed to read manifest file: {}", manifest_path.display())
                })?;

                let parsed: FlatpakManifest =
                    if manifest_path.extension().and_then(|s| s.to_str()) == Some("json") {
                        serde_json::from_str(&content).with_context(|| {
                            format!("Failed to parse JSON manifest: {}", manifest_path.display())
                        })?
                    } else {
                        serde_norway::from_str(&content).with_context(|| {
                            format!("Failed to parse YAML manifest: {}", manifest_path.display())
                        })?
                    };
                parsed
            }
            Err(_) => {
                let mut synthetic = FlatpakManifest::default();
                synthetic.id = synthetic.resolve_app_id(workspace_path);
                synthetic.buildsystem = Some("meson".to_string());

                if let Ok(content) = fs::read_to_string(workspace_path.join("snap/snapcraft.yaml"))
                    && let Ok(val) = serde_norway::from_str::<serde_json::Value>(&content)
                    && synthetic.command.is_none()
                {
                    synthetic.command = val.get("name").and_then(|v| v.as_str()).map(String::from);
                }

                synthetic
            }
        };

        manifest.sanitize_internal_id();

        manifest.validate_meson(workspace_path)?;

        Ok(manifest)
    }

    pub fn get_app_id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// Resolves the App ID by prioritizing:
    /// 1. Direct manifest `id` / `app-id` field
    /// 2. Reverse-DNS AppStream metadata files (`*.metainfo.xml`, `*.appdata.xml`)
    /// 3. `app_id` / `application_id` declarations inside `meson.build`
    pub fn resolve_app_id(&self, workspace_path: &Path) -> Option<String> {
        self.get_app_id()
            .filter(|id| !id.is_empty())
            .map(String::from)
            .or_else(|| find_app_id_from_metainfo(workspace_path))
            .or_else(|| find_app_id_from_meson(workspace_path))
            .map(|id| Self::sanitize_app_id_str(&id).to_string())
    }

    fn sanitize_internal_id(&mut self) {
        if let Some(ref mut id) = self.id {
            let sanitized = Self::sanitize_app_id_str(id);
            if sanitized != id {
                *id = sanitized.to_string();
            }
        }
    }

    fn sanitize_app_id_str(id: &str) -> &str {
        let trimmed = id.trim();

        trimmed
            .strip_suffix(".Devel")
            .or_else(|| trimmed.strip_suffix(".devel"))
            .unwrap_or(trimmed)
    }

    pub fn validate_meson(&self, workspace_path: &Path) -> Result<()> {
        let meson_build = workspace_path.join("meson.build");
        if !meson_build.exists() {
            bail!(
                "Missing 'meson.build' file in workspace root ({}). This repository is not a Meson project.",
                workspace_path.display()
            );
        }

        if let Some(ref bs) = self.buildsystem
            && bs.to_lowercase() == "meson"
        {
            return Ok(());
        }

        if let Some(ref modules) = self.modules {
            let resolved_id = self.resolve_app_id(workspace_path);
            if has_meson_app_module(modules, resolved_id.as_deref(), workspace_path) {
                return Ok(());
            }
        } else {
            return Ok(());
        }

        bail!(
            "Could not find a Meson-based main application module in the Flatpak manifest. flatpak2spec currently requires a Meson build system."
        );
    }
}

fn find_manifest_file(workspace_path: &Path) -> Result<std::path::PathBuf> {
    let candidates = utils::find_matching_files(workspace_path, &[".json", ".yaml", ".yml"]);

    for path in candidates {
        if is_likely_manifest(&path) {
            return Ok(path);
        }
    }

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

        if filename.matches('.').count() >= 2 || name_lower.contains("manifest") {
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

fn has_meson_app_module(modules: &[Module], app_id: Option<&str>, workspace_path: &Path) -> bool {
    let total_modules = modules.len();

    for module in modules {
        let Ok(detail) = module.resolve(workspace_path) else {
            continue;
        };

        let bs = detail.buildsystem.as_deref().unwrap_or("meson");

        if bs.to_lowercase() == "meson"
            && is_main_app_module(&detail, app_id, total_modules, workspace_path)
        {
            return true;
        }

        if let Some(ref child_modules) = detail.modules
            && has_meson_app_module(child_modules, app_id, workspace_path)
        {
            return true;
        }
    }

    false
}

fn is_main_app_module(
    module: &ModuleDetail,
    app_id: Option<&str>,
    total_modules: usize,
    workspace_path: &Path,
) -> bool {
    let id_lower = app_id.map(|s| s.to_lowercase());
    let name_lower = module.name.to_lowercase();

    if let Some(ref id) = id_lower {
        let last_segment = id.split('.').next_back().unwrap_or(id).to_lowercase();
        if name_lower == *id || name_lower == last_segment {
            return true;
        }
    }

    if let Some(ref sources) = module.sources {
        for source in sources {
            let Ok(detail) = source.resolve(workspace_path) else {
                continue;
            };

            match detail {
                SourceDetail::Dir { path } => {
                    if path == "." || path == "./" || path == ".." || path == "../" {
                        return true;
                    }
                }
                SourceDetail::Archive { url } | SourceDetail::Git { url } => {
                    if let Some(ref id) = id_lower {
                        let last_segment = id.split('.').next_back().unwrap_or(id).to_lowercase();
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
                if (trimmed.starts_with("app_id") || trimmed.starts_with("application_id"))
                    && let Some(val) = trimmed.split('=').nth(1)
                {
                    let clean = val.trim().trim_matches('\'').trim_matches('"');
                    if clean.matches('.').count() >= 2 {
                        return Some(clean.to_string());
                    }
                }
            }
        }
    }
    None
}

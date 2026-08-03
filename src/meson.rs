// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Infiniti151

use crate::utils;
use anyhow::Result;
use regex::Regex;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct GnomePostInstall {
    pub glib_compile_schemas: bool,
    pub gtk_update_icon_cache: bool,
    pub update_desktop_database: bool,
}

#[derive(Debug, Clone, Default)]
pub struct MesonModules {
    pub has_i18n: bool,
    pub has_gnome: bool,
    pub has_blueprint: bool,
    pub gnome_post_install: GnomePostInstall,
    pub subdirs: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct MesonProject {
    pub name: Option<String>,
    pub license: Option<String>,
    pub languages: Vec<String>,
    pub pkgconfig_deps: BTreeSet<String>,
    pub required_tools: BTreeSet<String>,
    pub installed_executables: BTreeSet<String>,
    pub meson_min_version: Option<String>,
    pub modules: MesonModules,

    // --- Python Flags ---
    pub needs_python_build_tool: bool,
    pub is_python_app: bool,
    pub has_pygobject: bool,
}

impl MesonProject {
    pub fn parse_from_workspace(workspace_path: &Path) -> Result<Self> {
        let mut project = Self::default();

        // 1. Scan primary workspace files
        let meson_files = utils::find_matching_files(workspace_path, &["meson.build"]);
        let root_meson = workspace_path.join("meson.build");

        for file_path in meson_files {
            if let Ok(content) = fs::read_to_string(&file_path) {
                let is_root = file_path == root_meson;
                project.parse_content(&content, is_root);
            }
        }

        // 2. Extract app-level license strictly from the root workspace
        if project.license.is_none() && utils::has_metainfo_file(workspace_path) {
            project.license = Self::extract_license_from_metainfo(workspace_path);
        }

        // 3. Scan subprojects (e.g., subprojects/magpie)
        let subprojects_dir = workspace_path.join("subprojects");
        if subprojects_dir.is_dir() {
            project.parse_subproject_dependencies(&subprojects_dir);
        }

        Ok(project)
    }

    /// Recursively collects dependencies across subprojects without modifying top-level app metadata.
    fn parse_subproject_dependencies(&mut self, subprojects_dir: &Path) {
        let subproject_mesons = Self::find_all_subproject_meson_files(subprojects_dir);

        for file_path in subproject_mesons {
            if let Ok(content) = fs::read_to_string(&file_path) {
                // Pass false for is_root, but parse_content will still add to `self.installed_executables`
                self.parse_content(&content, false);
            }
        }
    }

    fn find_all_subproject_meson_files(dir: &Path) -> Vec<PathBuf> {
        let mut matches = Vec::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                if name.starts_with('.') || name == "build" || name == "_build" {
                    continue;
                }

                if path.is_dir() {
                    matches.extend(Self::find_all_subproject_meson_files(&path));
                } else if name == "meson.build" {
                    matches.push(path);
                }
            }
        }
        matches
    }

    fn parse_content(&mut self, content: &str, is_root: bool) {
        if is_root {
            // Extract project name
            let name_re = Regex::new(r#"(?i)project\s*\(\s*['"]([^'"]+)['"]"#).unwrap();
            if let Some(caps) = name_re.captures(content) {
                let raw_name = &caps[1];
                let clean_name = if raw_name.contains('.') {
                    raw_name
                        .split('.')
                        .next_back()
                        .unwrap_or(raw_name)
                        .to_lowercase()
                } else {
                    raw_name.to_lowercase()
                };
                self.name = Some(clean_name);
            }

            // Extract minimum meson version
            let meson_ver_re =
                Regex::new(r#"(?i)meson_version\s*:\s*['"]\s*(?:>=)?\s*([^'"]+)['"]"#).unwrap();
            if let Some(caps) = meson_ver_re.captures(content) {
                self.meson_min_version = Some(caps[1].trim().to_string());
            }

            // Extract license
            let lic_re = Regex::new(r#"(?i)license\s*:\s*['"]([^'"]+)['"]"#).unwrap();
            if let Some(caps) = lic_re.captures(content) {
                self.license = Some(caps[1].to_string());
            }

            // Extract languages
            let lang_re = Regex::new(r#"project\s*\([^)]*?\)"#).unwrap();
            if let Some(mat) = lang_re.find(content) {
                let string_re = Regex::new(r#"['"]([^'"]+)['"]"#).unwrap();
                let mut matches: Vec<String> = string_re
                    .captures_iter(mat.as_str())
                    .map(|cap| cap[1].to_string())
                    .collect();

                if !matches.is_empty() {
                    matches.remove(0); // Skip app name
                    self.languages = matches
                        .into_iter()
                        .filter(|s| !s.contains(':') && !s.contains('='))
                        .collect();
                }
            }
        }

        // --- Extract Executables & Custom Targets ---

        // 1. Native executables
        let exe_re = Regex::new(r#"(?s)executable\s*\(\s*['"]([^'"]+)['"]"#).unwrap();
        for cap in exe_re.captures_iter(content) {
            let exe_name = cap[1].to_string();

            // Find the boundary of this specific executable call to check install status
            if let Some(mat) = cap.get(0) {
                let start = mat.start();
                // Grab a window of characters after the call to check arguments
                let sample = &content[start..content.len().min(start + 500)];

                let is_explicit_no_install = sample.contains("install : false")
                    || sample.contains("install: false")
                    || sample.contains("build_by_default : false");

                if !is_explicit_no_install {
                    self.installed_executables.insert(exe_name);
                }
            }
        }

        // 2. Custom targets
        let custom_target_re =
            Regex::new(r#"(?s)custom_target\s*\(\s*(?:['"]([^'"]+)['"]\s*,\s*)?(.*?)\)"#).unwrap();
        let output_str_re = Regex::new(r#"output\s*:\s*(?:\[\s*)?['"]([^'"]+)['"]"#).unwrap();

        for cap in custom_target_re.captures_iter(content) {
            let block = &cap[2];

            // Catch custom targets with `install: true` OR `install_dir`
            let is_installed = block.contains("install : true")
                || block.contains("install: true")
                || block.contains("install_dir");

            if is_installed && let Some(out_cap) = output_str_re.captures(block) {
                let out_name = out_cap[1].trim();
                if out_name != "@OUTPUT@" {
                    self.installed_executables.insert(out_name.to_string());
                }
            }
        }

        // Extract subdirs
        let subdir_re = Regex::new(r#"(?i)subdir\s*\(\s*['"]([^'"]+)['"]\s*\)"#).unwrap();
        for cap in subdir_re.captures_iter(content) {
            let sub = cap[1].trim().to_string();
            if !self.modules.subdirs.contains(&sub) {
                self.modules.subdirs.push(sub);
            }
        }

        // Runtime Python checks
        let has_python_import =
            content.contains("import('python')") || content.contains("import('python3')");
        let has_python_dep =
            content.contains("dependency('python") || content.contains("dependency('python3");
        let has_python_inst = content.contains("python.find_installation");

        if has_python_import || has_python_dep || has_python_inst {
            self.is_python_app = true;
        }

        // PyGObject
        if content.contains("dependency('pygobject")
            || content.contains("find_program('pygobject")
            || content.contains("gi.repository")
        {
            self.is_python_app = true;
            self.has_pygobject = true;
        }

        // Build-time Python
        let has_python_script = content
            .lines()
            .map(|line| line.split('#').next().unwrap_or(""))
            .any(|line| {
                (line.contains(".py'") || line.contains(".py\""))
                    && (line.contains("find_program")
                        || line.contains("custom_target")
                        || line.contains("run_command")
                        || line.contains("post_install"))
            });

        if has_python_script || content.contains("find_program('python3')") {
            self.needs_python_build_tool = true;
        }

        // Imported modules
        let import_re = Regex::new(r#"(?i)\w+\s*=\s*import\s*\(\s*['"]([^'"]+)['"]\s*\)"#).unwrap();
        for cap in import_re.captures_iter(content) {
            match cap[1].to_lowercase().as_str() {
                "i18n" => self.modules.has_i18n = true,
                "gnome" => self.modules.has_gnome = true,
                "python" | "python3" => {
                    self.is_python_app = true;
                    self.needs_python_build_tool = true;
                }
                _ => {}
            }
        }

        // Pkgconfig dependencies
        let dep_re = Regex::new(
    r#"(?i)dependency\s*\(\s*['"]([^'"]+)['"](?:\s*,\s*version\s*:\s*['"]\s*(?:>=)?\s*([^'"]+)['"])?"#,
)
.unwrap();

        for cap in dep_re.captures_iter(content) {
            let dep_name = &cap[1];

            if dep_name.contains("pygobject") {
                self.is_python_app = true;
                self.has_pygobject = true;
            }

            let dep_str = if let Some(ver) = cap.get(2) {
                format!("pkgconfig({}) >= {}", dep_name, ver.as_str().trim())
            } else {
                format!("pkgconfig({})", dep_name)
            };
            self.pkgconfig_deps.insert(dep_str);
        }

        // find_program requirements
        let prog_re = Regex::new(r#"(?i)find_program\s*\(\s*['"]([^'"]+)['"]"#).unwrap();
        for cap in prog_re.captures_iter(content) {
            let prog_name = &cap[1];

            let is_script = prog_name.ends_with(".py")
                || prog_name.ends_with(".sh")
                || prog_name.ends_with(".pl")
                || prog_name.ends_with(".rb");

            if !prog_name.contains('/')
                && !prog_name.starts_with('.')
                && !prog_name.starts_with("python")
                && !is_script
            {
                self.required_tools.insert(prog_name.to_string());
            }
        }

        // Check blueprint compiler
        if content.contains("blueprint-compiler") {
            self.modules.has_blueprint = true;
        }

        // Parse gnome.post_install options
        if self.modules.has_gnome {
            let post_install_re = Regex::new(r"(?s)gnome\.post_install\s*\((.*?)\)").unwrap();
            if let Some(caps) = post_install_re.captures(content) {
                let block = &caps[1];
                let bool_arg = |key: &str| {
                    Regex::new(&format!(r#"{}\s*:\s*(true|false)"#, key))
                        .ok()
                        .and_then(|re| re.captures(block))
                        .map(|c| &c[1] == "true")
                        .unwrap_or(false)
                };

                self.modules.gnome_post_install = GnomePostInstall {
                    glib_compile_schemas: bool_arg("glib_compile_schemas"),
                    gtk_update_icon_cache: bool_arg("gtk_update_icon_cache"),
                    update_desktop_database: bool_arg("update_desktop_database"),
                };
            }
        }
    }

    fn extract_license_from_metainfo(workspace_path: &Path) -> Option<String> {
        let re = Regex::new(r"<project_license>(.*?)</project_license>").ok()?;
        let files = utils::find_matching_files(workspace_path, &["metainfo.xml", "appdata.xml"]);

        for path in files {
            if let Ok(content) = fs::read_to_string(path)
                && let Some(caps) = re.captures(&content)
            {
                let lic = caps[1].trim().to_string();
                if !lic.is_empty() {
                    return Some(lic);
                }
            }
        }
        None
    }

    pub fn has_subdir(&self, name: &str) -> bool {
        self.modules
            .subdirs
            .iter()
            .any(|s| s.eq_ignore_ascii_case(name))
    }

    pub fn has_po_subdir(&self) -> bool {
        self.has_subdir("po") || self.has_subdir("po-body")
    }

    pub fn is_rust_project(&self, workspace_path: &Path) -> bool {
        if workspace_path.join("Cargo.toml").exists()
            || workspace_path.join("src/Cargo.toml").exists()
            || self
                .languages
                .iter()
                .any(|lang| lang.eq_ignore_ascii_case("rust"))
        {
            return true;
        }

        let subprojects_dir = workspace_path.join("subprojects");
        if subprojects_dir.is_dir()
            && let Ok(entries) = fs::read_dir(subprojects_dir)
        {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.join("Cargo.toml").exists() {
                    return true;
                }
            }
        }

        false
    }

    pub fn is_noarch(&self) -> bool {
        let compiled_languages = [
            "c", "cpp", "cuda", "cython", "d", "objc", "objcpp", "fortran", "cs", "swift", "vala",
            "rust", "nasm", "masm",
        ];

        !self
            .languages
            .iter()
            .any(|lang| compiled_languages.contains(&lang.to_lowercase().as_str()))
    }
}

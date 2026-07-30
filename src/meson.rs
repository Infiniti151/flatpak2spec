// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Infiniti151

use crate::utils;
use anyhow::Result;
use regex::Regex;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

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
    pub meson_min_version: Option<String>,
    pub has_python: bool,
    pub has_pygobject: bool,
    pub modules: MesonModules,
}

impl MesonProject {
    pub fn parse_from_workspace(workspace_path: &Path) -> Result<Self> {
        let mut project = Self::default();

        let meson_files = utils::find_matching_files(workspace_path, &["meson.build"]);
        let root_meson = workspace_path.join("meson.build");

        for file_path in meson_files {
            if let Ok(content) = fs::read_to_string(&file_path) {
                let is_root = file_path == root_meson;
                project.parse_content(&content, is_root);
            }
        }

        if project.license.is_none() && utils::has_metainfo_file(workspace_path) {
            project.license = Self::extract_license_from_metainfo(workspace_path);
        }

        Ok(project)
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
                        .last()
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

        // Extract subdirs
        let subdir_re = Regex::new(r#"(?i)subdir\s*\(\s*['"]([^'"]+)['"]\s*\)"#).unwrap();
        for cap in subdir_re.captures_iter(content) {
            let sub = cap[1].trim().to_string();
            if !self.modules.subdirs.contains(&sub) {
                self.modules.subdirs.push(sub);
            }
        }

        // Check for Python / PyGObject
        if content.contains("import('python')")
            || content.contains("find_installation")
            || content.contains("python3")
        {
            self.has_python = true;
        }

        if content.contains("pygobject") || content.contains("gi.repository") {
            self.has_python = true;
            self.has_pygobject = true;
        }

        // Extract imported modules
        let import_re = Regex::new(r#"(?i)\w+\s*=\s*import\s*\(\s*['"]([^'"]+)['"]\s*\)"#).unwrap();
        for cap in import_re.captures_iter(content) {
            match cap[1].to_lowercase().as_str() {
                "i18n" => self.modules.has_i18n = true,
                "gnome" => self.modules.has_gnome = true,
                "python" => self.has_python = true,
                _ => {}
            }
        }

        // Extract pkgconfig dependencies
        let dep_re = Regex::new(
            r#"(?i)dependency\s*\(\s*['"]([^'"]+)['"](?:\s*,\s*version\s*:\s*['"]\s*(?:>=)?\s*([^'"]+)['"])?"#,
        )
        .unwrap();

        for cap in dep_re.captures_iter(content) {
            let dep_name = &cap[1];
            if dep_name.contains("pygobject") {
                self.has_python = true;
                self.has_pygobject = true;
            }

            let dep_str = if let Some(ver) = cap.get(2) {
                format!("pkgconfig({}) >= {}", dep_name, ver.as_str().trim())
            } else {
                format!("pkgconfig({})", dep_name)
            };
            self.pkgconfig_deps.insert(dep_str);
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
            if let Ok(content) = fs::read_to_string(path) {
                if let Some(caps) = re.captures(&content) {
                    let lic = caps[1].trim().to_string();
                    if !lic.is_empty() {
                        return Some(lic);
                    }
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
        workspace_path.join("Cargo.toml").exists()
            || workspace_path.join("src/Cargo.toml").exists()
            || self
                .languages
                .iter()
                .any(|lang| lang.eq_ignore_ascii_case("rust"))
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

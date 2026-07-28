// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Infiniti151

use crate::meson::MesonProject;
use crate::utils::{check_file_extension, has_desktop_file, has_metainfo_file};
use std::collections::BTreeSet;
use std::path::Path;

pub struct DepsSection;

impl DepsSection {
    pub fn generate(meson: &MesonProject, workspace_path: &Path) -> String {
        let mut build_requires = BTreeSet::new();
        let mut requires = BTreeSet::new();

        // 1. Core Meson build requirements
        if let Some(version) = &meson.meson_min_version {
            build_requires.insert(format!("meson >= {}", version));
        } else {
            build_requires.insert("meson".to_string());
        }
        build_requires.insert("ninja-build".to_string());

        // 2. Python detection
        let contains_python_files = Self::has_python_files(workspace_path);

        if meson.has_python || contains_python_files {
            build_requires.insert("python3-devel".to_string());
            requires.insert("python3".to_string());

            if meson.has_pygobject || meson.modules.has_gnome || contains_python_files {
                requires.insert("python3-gobject".to_string());
            }
        }

        // 3. Language compilers
        for lang in &meson.languages {
            match lang.to_lowercase().as_str() {
                "c" => {
                    build_requires.insert("gcc".to_string());
                }
                "cpp" => {
                    build_requires.insert("gcc-c++".to_string());
                }
                "vala" => {
                    build_requires.insert("valac".to_string());
                }
                "rust" => {
                    build_requires.insert("cargo".to_string());
                    build_requires.insert("rustc".to_string());
                }
                _ => {}
            }
        }

        if meson.is_rust_project(workspace_path) {
            build_requires.insert("cargo".to_string());
            build_requires.insert("rustc".to_string());
            build_requires.insert("cargo-rpm-macros".to_string());
        }

        // 4. Direct pkgconfig dependencies
        for dep in &meson.pkgconfig_deps {
            build_requires.insert(dep.clone());
        }

        // 5. Blueprint compiler check
        if meson.modules.has_blueprint || Self::has_blueprint_files(workspace_path) {
            build_requires.insert("blueprint-compiler".to_string());
        }

        // 6. Localization tools (only added if i18n module or po/ directory exists)
        if meson.modules.has_i18n || meson.has_po_subdir() {
            build_requires.insert("gettext".to_string());
            build_requires.insert("glibc-langpack-en".to_string());
        }

        if meson.modules.gnome_post_install.gtk_update_icon_cache {
            build_requires.insert("gtk-update-icon-cache".to_string());
        }

        // 7. Desktop & AppStream metadata validators
        if has_desktop_file(workspace_path) {
            build_requires.insert("desktop-file-utils".to_string());
        }

        if has_metainfo_file(workspace_path) {
            build_requires.insert("libappstream-glib".to_string());
        }

        // Format output string
        let mut output = String::new();

        // BuildRequires block (Sort meson and ninja-build to top)
        if !build_requires.is_empty() {
            let mut sorted_br: Vec<String> = build_requires.into_iter().collect();
            sorted_br.sort_by(|a, b| {
                let is_build_tool = |s: &str| s.starts_with("meson") || s.starts_with("ninja");
                match (is_build_tool(a), is_build_tool(b)) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.cmp(b),
                }
            });

            for req in sorted_br {
                output.push_str(&format!("BuildRequires:  {}\n", req));
            }
        }

        // Requires block
        if !requires.is_empty() {
            output.push('\n');
            for req in requires {
                output.push_str(&format!("Requires:       {}\n", req));
            }
        }

        output
    }

    fn has_python_files(dir: &Path) -> bool {
        check_file_extension(dir, ".py")
    }

    fn has_blueprint_files(dir: &Path) -> bool {
        check_file_extension(dir, ".blp")
    }
}

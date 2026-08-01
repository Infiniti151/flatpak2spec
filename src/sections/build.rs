// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Infiniti151

use crate::meson::MesonProject;
use crate::utils::{has_desktop_file, has_metainfo_file};
use std::path::Path;

pub struct BuildSection;

impl BuildSection {
    pub fn generate(meson: &MesonProject, workspace_path: &Path) -> String {
        let mut section = String::new();

        // %prep
        section.push_str("%prep\n");
        section.push_str("%forgesetup\n");

        if workspace_path.join(".gitmodules").exists() {
            section.push_str(
                "\n# Re-initialize git context for forge tarballs to resolve submodules\n\
                 if [ ! -d \".git\" ]; then\n\
                 \tgit init -q\n\
                 \tgit remote add origin %{forgeurl}\n\
                 \tgit fetch -q --depth 1 origin %{tag}\n\
                 \tgit checkout -q -f FETCH_HEAD\n\
                 fi\n\
                 git submodule update --init --recursive\n",
            );
        }

        section.push('\n');

        // %build
        section.push_str("%build\n");
        section.push_str("%meson\n");
        section.push_str("%meson_build\n\n");

        // %install
        section.push_str("%install\n");
        section.push_str("%meson_install\n");
        if meson.has_po_subdir() {
            section.push_str("%find_lang %{name}\n");
        }
        if meson.is_python_app {
            section.push_str("%py3_shebang_fix %{buildroot}%{_bindir}/%{name} %{buildroot}%{_datadir}/%{name}/\n");
        }
        section.push('\n');

        // %check
        section.push_str("%check\n");
        section.push_str("%meson_test\n");

        if has_desktop_file(workspace_path) {
            section
                .push_str("desktop-file-validate %{buildroot}%{_datadir}/applications/*.desktop\n");
        }

        if has_metainfo_file(workspace_path) {
            section.push_str("appstream-util validate-relax --nonet %{buildroot}%{_metainfodir}/*.metainfo.xml\n");
        }

        if meson.modules.gnome_post_install.glib_compile_schemas {
            section.push_str("glib-compile-schemas --dry-run --strict %{buildroot}%{_datadir}/glib-2.0/schemas/\n");
        }

        section.push('\n');

        section
    }
}

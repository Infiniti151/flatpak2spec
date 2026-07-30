// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Infiniti151

use crate::manifest::FlatpakManifest;
use crate::meson::MesonProject;
use crate::utils::{
    check_file_extension, detect_systemd_units, find_matching_files, has_desktop_file,
    has_metainfo_file,
};
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct FilesContext {
    pub binary_name: String,
    pub has_i18n: bool,
    pub has_desktop: bool,
    pub has_metainfo: bool,
    pub has_schemas: bool,
    pub has_dbus: bool,
    pub has_icons: bool,
    pub has_user_service: bool,
    pub has_system_service: bool,
    pub doc_files: Vec<String>,
    pub license_files: Vec<String>,
}

impl FilesContext {
    pub fn inspect(
        workspace: &Path,
        manifest: Option<&FlatpakManifest>,
        meson: Option<&MesonProject>,
    ) -> Self {
        let meson_name = meson.and_then(|m| m.name.as_deref());
        let manifest_id = manifest.and_then(|m| m.id.as_deref());
        let manifest_cmd = manifest.and_then(|m| m.command.as_deref());

        let binary_name = match manifest_cmd {
            Some(cmd) => {
                if Some(cmd) == meson_name {
                    "%{name}".to_string()
                } else if Some(cmd) == manifest_id {
                    "%{app_id}".to_string()
                } else {
                    cmd.to_string()
                }
            }
            None => "%{name}".to_string(),
        };

        let has_i18n = meson.map(|m| m.modules.has_i18n).unwrap_or(false)
            || check_file_extension(workspace, ".po");

        let has_desktop = has_desktop_file(workspace);
        let has_metainfo = has_metainfo_file(workspace);
        let has_schemas = check_file_extension(workspace, ".gschema.xml");
        let has_dbus = check_file_extension(workspace, ".service");
        let has_icons =
            check_file_extension(workspace, ".svg") || check_file_extension(workspace, ".png");

        let systemd = detect_systemd_units(workspace);
        let (doc_files, license_files) = scan_docs_and_licenses(workspace);

        Self {
            binary_name,
            has_i18n,
            has_desktop,
            has_metainfo,
            has_schemas,
            has_dbus,
            has_icons,
            has_user_service: systemd.has_user,
            has_system_service: systemd.has_system,
            doc_files,
            license_files,
        }
    }
}

/// Scans the workspace root for standard license and documentation files.
fn scan_docs_and_licenses(workspace: &Path) -> (Vec<String>, Vec<String>) {
    let patterns = ["license", "copying", "readme", "news", "changelog"];
    let matching_paths = find_matching_files(workspace, &patterns);

    let mut docs = Vec::new();
    let mut licenses = Vec::new();

    for path in matching_paths {
        // Only consider top-level workspace files
        if path.parent() == Some(workspace) {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                let upper = name.to_uppercase();
                if upper.starts_with("LICENSE") || upper.starts_with("COPYING") {
                    licenses.push(name.to_string());
                } else if upper.starts_with("README")
                    || upper.starts_with("NEWS")
                    || upper.starts_with("CHANGELOG")
                {
                    docs.push(name.to_string());
                }
            }
        }
    }

    docs.sort();
    licenses.sort();
    (docs, licenses)
}

pub fn generate_files_section(ctx: &FilesContext) -> String {
    let mut out = String::new();

    // 1. Header with language macro if i18n is enabled
    if ctx.has_i18n {
        out.push_str("%files -f %{name}.lang\n");
    } else {
        out.push_str("%files\n");
    }

    // 2. Licenses & Docs
    if !ctx.license_files.is_empty() {
        out.push_str(&format!("%license {}\n", ctx.license_files.join(" ")));
    }
    if !ctx.doc_files.is_empty() {
        out.push_str(&format!("%doc {}\n", ctx.doc_files.join(" ")));
    }

    // 3. Executable
    out.push_str(&format!("%{{_bindir}}/{}\n", ctx.binary_name));

    // 4. Data Directories
    out.push_str(&format!("%{{_datadir}}/{}\n", ctx.binary_name));

    if ctx.has_desktop {
        out.push_str("%{_datadir}/applications/%{app_id}.desktop\n");
    }
    if ctx.has_icons {
        out.push_str("%{_datadir}/icons/hicolor/*/apps/%{app_id}*\n");
    }
    if ctx.has_schemas {
        out.push_str("%{_datadir}/glib-2.0/schemas/*.gschema.xml\n");
    }
    if ctx.has_dbus {
        out.push_str("%{_datadir}/dbus-1/services/*.service\n");
    }

    // 5. MetaInfo
    if ctx.has_metainfo {
        out.push_str("%{_metainfodir}/%{app_id}.metainfo.xml\n");
    }

    // 6. Systemd Units
    if ctx.has_user_service {
        out.push_str("%{_userunitdir}/*.service\n");
    }
    if ctx.has_system_service {
        out.push_str("%{_unitdir}/*.service\n");
    }

    out
}

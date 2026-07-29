// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Infiniti151

use crate::utils;
use std::fs;
use std::path::Path;

pub struct ScriptletsSection;

impl ScriptletsSection {
    /// Generates RPM scriptlets (%post, %preun, %postun) for systemd services.
    ///
    /// Modern RPM distributions handle icon caches, GSettings schemas, and desktop databases
    /// automatically via RPM File Triggers. Scriptlets are only generated when systemd
    /// unit files (.service, .socket, .timer, etc.) are detected in the project workspace.
    /// D-Bus service activation files (e.g. [D-BUS Service] in dbus-1/services) are automatically ignored.
    pub fn generate(workspace_path: &Path) -> String {
        let mut scriptlets = String::new();

        // Target systemd unit file extension patterns
        let unit_patterns = [
            ".service",
            ".service.in",
            ".socket",
            ".socket.in",
            ".timer",
            ".timer.in",
            ".path",
            ".path.in",
            ".target",
            ".target.in",
        ];

        // Find matching paths using utils::find_matching_files
        let matching_paths = utils::find_matching_files(workspace_path, &unit_patterns);

        if matching_paths.is_empty() {
            return scriptlets;
        }

        let mut unit_files: Vec<String> = Vec::new();

        for path in matching_paths {
            let path_str = path.to_string_lossy().to_lowercase();

            // 1. Quick path check: ignore files in dbus directories
            if path_str.contains("dbus-1") || path_str.contains("dbus-services") {
                continue;
            }

            // 2. Content check: verify it is not a D-Bus activation service file
            if let Ok(content) = fs::read_to_string(&path) {
                if content.contains("[D-BUS Service]") {
                    continue;
                }
            }

            // 3. Extract clean unit file name (e.g., "my-daemon.service")
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                let clean_name = file_name.strip_suffix(".in").unwrap_or(file_name);

                if !unit_files.contains(&clean_name.to_string()) {
                    unit_files.push(clean_name.to_string());
                }
            }
        }

        if unit_files.is_empty() {
            return scriptlets;
        }

        unit_files.sort();
        let unit_args = unit_files.join(" ");

        // %post block
        scriptlets.push_str("%post\n");
        scriptlets.push_str(&format!("%systemd_post {}\n\n", unit_args));

        // %preun block
        scriptlets.push_str("%preun\n");
        scriptlets.push_str(&format!("%systemd_preun {}\n\n", unit_args));

        // %postun block
        scriptlets.push_str("%postun\n");
        scriptlets.push_str(&format!("%systemd_postun_with_restart {}\n\n", unit_args));

        scriptlets
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Infiniti151

use crate::utils;
use std::path::Path;

pub struct ScriptletsSection;

impl ScriptletsSection {
    pub fn generate(workspace_path: &Path) -> String {
        let mut scriptlets = String::new();

        // 1. Detect systemd unit files using shared utility
        let systemd = utils::detect_systemd_units(workspace_path);

        if systemd.unit_names.is_empty() {
            return scriptlets;
        }

        let unit_args = systemd.unit_names.join(" ");

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

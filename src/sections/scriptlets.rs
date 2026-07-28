// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Infiniti151

use crate::meson::MesonProject;

pub struct ScriptletsSection;

impl ScriptletsSection {
    /// Generates RPM scriptlets (%post, %postun, %posttrans) based on Meson post-install flags.
    pub fn generate(meson: &MesonProject) -> String {
        let mut scriptlets = String::new();
        let hooks = &meson.modules.gnome_post_install;

        // Check if any post-install actions are requested
        if !hooks.glib_compile_schemas
            && !hooks.gtk_update_icon_cache
            && !hooks.update_desktop_database
        {
            return scriptlets;
        }

        let mut post_lines = Vec::new();
        let mut postun_lines = Vec::new();
        let mut posttrans_lines = Vec::new();

        // 1. Desktop database cache
        if hooks.update_desktop_database {
            post_lines.push("%{_bindir}/update-desktop-database &> /dev/null || :");
            postun_lines.push("%{_bindir}/update-desktop-database &> /dev/null || :");
        }

        // 2. GTK icon theme cache
        if hooks.gtk_update_icon_cache {
            post_lines.push(
                "%{_bindir}/gtk-update-icon-cache -f -t %{_datadir}/icons/hicolor &> /dev/null || :",
            );
            postun_lines.push(
                "%{_bindir}/gtk-update-icon-cache -f -t %{_datadir}/icons/hicolor &> /dev/null || :",
            );
        }

        // 3. GLib GSettings schemas
        if hooks.glib_compile_schemas {
            posttrans_lines.push(
                "%{_bindir}/glib-compile-schemas %{_datadir}/glib-2.0/schemas &> /dev/null || :",
            );
        }

        // Format %post block
        if !post_lines.is_empty() {
            scriptlets.push_str("%post\n");
            for line in post_lines {
                scriptlets.push_str(line);
                scriptlets.push('\n');
            }
            scriptlets.push('\n');
        }

        // Format %postun block
        if !postun_lines.is_empty() {
            scriptlets.push_str("%postun\n");
            for line in postun_lines {
                scriptlets.push_str(line);
                scriptlets.push('\n');
            }
            scriptlets.push('\n');
        }

        // Format %posttrans block
        if !posttrans_lines.is_empty() {
            scriptlets.push_str("%posttrans\n");
            for line in posttrans_lines {
                scriptlets.push_str(line);
                scriptlets.push('\n');
            }
            scriptlets.push('\n');
        }

        scriptlets
    }
}

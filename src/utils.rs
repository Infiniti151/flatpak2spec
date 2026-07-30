// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Infiniti151

use colored::Colorize;
use std::path::{Path, PathBuf};

/// Recursively searches a directory for files matching a specific extension or name snippet,
/// including `.in` template files (e.g. `.desktop.in`).
pub fn check_file_extension(dir: &Path, ext: &str) -> bool {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return false,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Skip hidden dotfiles/directories (.git, .build, etc.)
        if name.starts_with('.') {
            continue;
        }

        if path.is_dir() {
            if check_file_extension(&path, ext) {
                return true;
            }
        } else if name.ends_with(ext)
            || name.ends_with(&format!("{}.in", ext))
            || name.contains(ext)
        {
            return true;
        }
    }
    false
}

/// Checks if the workspace contains desktop files or templates (.desktop / .desktop.in).
pub fn has_desktop_file(dir: &Path) -> bool {
    check_file_extension(dir, ".desktop")
}

/// Checks if the workspace contains AppStream/MetaInfo XML files or templates.
pub fn has_metainfo_file(dir: &Path) -> bool {
    check_file_extension(dir, "metainfo.xml") || check_file_extension(dir, "appdata.xml")
}

/// Recursively finds all file paths matching a given set of pattern strings (e.g., [".desktop", "metainfo.xml"]).
pub fn find_matching_files(dir: &Path, patterns: &[&str]) -> Vec<PathBuf> {
    let mut matches = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return matches,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if name.starts_with('.') {
            continue;
        }

        if path.is_dir() {
            matches.extend(find_matching_files(&path, patterns));
        } else {
            let lower = name.to_lowercase();
            if patterns.iter().any(|pat| lower.contains(pat)) {
                matches.push(path);
            }
        }
    }
    matches
}

#[derive(Debug, Clone, Default)]
pub struct SystemdUnits {
    pub has_user: bool,
    pub has_system: bool,
    pub unit_names: Vec<String>,
}

/// Detects real systemd unit files in the workspace (ignoring D-Bus service files),
/// and extracts both their scope (User/System) and their clean filenames.
pub fn detect_systemd_units(workspace: &Path) -> SystemdUnits {
    let mut units = SystemdUnits::default();

    // Base patterns (find_matching_files uses `.contains()`, so this covers `.in` variants too)
    let unit_patterns = [".service", ".socket", ".timer", ".path", ".target"];
    let matching_paths = find_matching_files(workspace, &unit_patterns);

    for path in matching_paths {
        let path_str = path.to_string_lossy().to_lowercase();

        // 1. Ignore D-Bus service activation directories
        if path_str.contains("dbus-1") || path_str.contains("dbus-services") {
            continue;
        }

        if let Ok(content) = std::fs::read_to_string(&path) {
            // 2. Ignore D-Bus service activation files by content
            if content.contains("[D-BUS Service]") {
                continue;
            }

            // 3. Determine scope
            if path_str.contains("/user/") || path_str.contains("user-") {
                units.has_user = true;
            } else if path_str.contains("/system/")
                || content.contains("WantedBy=multi-user.target")
                || content.contains("User=root")
            {
                units.has_system = true;
            } else {
                units.has_user = true; // Default fallback
            }
        } else {
            // Fallback if file read fails but it matched the extension
            units.has_user = true;
        }

        // 4. Extract clean unit file name (e.g., "my-daemon.service")
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            let clean_name = file_name.strip_suffix(".in").unwrap_or(file_name);
            if !units.unit_names.contains(&clean_name.to_string()) {
                units.unit_names.push(clean_name.to_string());
            }
        }
    }

    units.unit_names.sort();
    units
}

pub fn print_info(message: &str) {
    println!("{}", message.yellow());
}

pub fn print_success(message: &str) {
    println!("{}", message.green());
}

pub fn print_error(message: &str) {
    eprintln!("{}", message.red());
}

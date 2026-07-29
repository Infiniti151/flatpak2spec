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

pub fn print_info(message: &str) {
    println!("{}", message.yellow());
}

pub fn print_success(message: &str) {
    println!("{}", message.green());
}

pub fn print_error(message: &str) {
    eprintln!("{}", message.red());
}

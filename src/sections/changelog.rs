// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Infiniti151

use crate::utils;
use chrono::Utc;
use roxmltree::Document;

/// Context containing all information needed to generate the RPM %changelog section.
#[derive(Debug, Clone)]
pub struct ChangelogContext {
    pub packager_name: String,
    pub packager_email: String,
    pub version: String,
    pub release: String,
}

impl ChangelogContext {
    pub fn new(packager_name: &str, packager_email: &str, version: &str, release: &str) -> Self {
        Self {
            packager_name: packager_name.to_string(),
            packager_email: packager_email.to_string(),
            version: version.to_string(),
            release: release.to_string(),
        }
    }

    /// Generates a single RPM `%changelog` entry.
    ///
    /// If `metainfo_xml_content` is provided, it parses `<release version="<target_version>">`
    /// and formats its `<description>` list items (truncated to a max of 10 items).
    /// If missing or not found, it falls back to: `- Update to version <version>`
    pub fn generate(&self, metainfo_xml_content: Option<&str>) -> String {
        let mut output = String::from("%changelog\n");

        // Format date to standard RPM header format: e.g., "Fri Jul 31 2026"
        let date_str = Utc::now().format("%a %b %d %Y").to_string();

        // Standard RPM Changelog Header Line
        output.push_str(&format!(
            "* {} {} <{}> - {}-{}\n",
            date_str, self.packager_name, self.packager_email, self.version, self.release
        ));

        let mut notes_found = false;

        if let Some(xml) = metainfo_xml_content
            && let Some(notes) = extract_release_notes_from_xml(xml, &self.version)
            && !notes.is_empty()
        {
            const MAX_LINES: usize = 10;
            let is_truncated = notes.len() > MAX_LINES;

            for line in notes.iter().take(MAX_LINES) {
                output.push_str(&format!("- {}\n", line));
            }

            if is_truncated {
                output.push_str("- ... see upstream for full release notes\n");
            }

            notes_found = true;
        }

        // Fallback if no XML was provided or matching release notes were not found
        if !notes_found {
            output.push_str(&format!("- Update to version {}\n", self.version));
        }

        output
    }
}

/// Parses AppStream metainfo XML and extracts list items or text from `<release version="...">`
fn extract_release_notes_from_xml(xml_content: &str, target_version: &str) -> Option<Vec<String>> {
    let doc = Document::parse(xml_content).ok()?;

    let release_node = doc.descendants().find(|n| {
        n.is_element()
            && n.tag_name().name() == "release"
            && n.attribute("version") == Some(target_version)
    })?;

    let formatted_notes = utils::format_appstream_node(release_node);
    if formatted_notes.trim().is_empty() {
        None
    } else {
        let notes: Vec<String> = formatted_notes
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .map(|l| {
                // Strip existing bullet markers from AppStream formatting so generate() handles '- '
                let clean_line = l
                    .strip_prefix("- ")
                    .or_else(|| l.strip_prefix("* "))
                    .or_else(|| l.strip_prefix("• "))
                    .unwrap_or(l)
                    .trim();

                clean_line.to_string()
            })
            .collect();

        if notes.is_empty() { None } else { Some(notes) }
    }
}

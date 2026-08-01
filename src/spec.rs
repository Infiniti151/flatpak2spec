// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Infiniti151

use crate::Cli;
use crate::manifest::FlatpakManifest;
use crate::meson::MesonProject;
use crate::repository::RepoResolver;
use crate::sections::build::BuildSection;
use crate::sections::changelog::ChangelogContext;
use crate::sections::deps::DepsSection;
use crate::sections::description::DescriptionSection;
use crate::sections::files::{FilesContext, generate_files_section};
use crate::sections::header::HeaderSection;
use crate::sections::scriptlets::ScriptletsSection;
use crate::utils::find_metainfo_file;
use std::path::Path;

pub struct SpecGenerator;

impl SpecGenerator {
    pub fn generate(
        manifest: &FlatpakManifest,
        meson: &MesonProject,
        workspace_path: &Path,
        cli: &Cli,
    ) -> String {
        let mut spec = String::new();

        // Resolve canonical web URL for forge-srpm-macros (%global forgeurl)
        let canonical_url = RepoResolver::resolve_web_url(&workspace_path);

        // 1. Preamble / Header block
        let header = HeaderSection::generate(
            manifest,
            meson,
            workspace_path,
            &canonical_url,
            cli.bug_url.as_deref(),
        );
        spec.push_str(&header);

        if !spec.ends_with("\n\n") {
            spec.push('\n');
        }

        // 2. Dependencies (BuildRequires & Requires)
        let deps = DepsSection::generate(meson, workspace_path);
        spec.push_str(&deps);

        if !deps.ends_with("\n\n") {
            spec.push('\n');
        }

        // 3. %description (DescriptionSection handles its own header title)
        let description = DescriptionSection::generate(workspace_path, &header);
        spec.push_str(&description);

        if !description.ends_with("\n\n") {
            spec.push('\n');
        }

        // 4. Build stages (%prep, %build, %install, %check)
        let build = BuildSection::generate(meson, workspace_path);
        spec.push_str(&build);

        if !build.ends_with("\n\n") {
            spec.push('\n');
        }

        // 5. Scriptlets (%post, %postun, %posttrans)
        let scriptlets = ScriptletsSection::generate(workspace_path);
        if !scriptlets.is_empty() {
            spec.push_str(&scriptlets);
            if !scriptlets.ends_with("\n\n") {
                spec.push('\n');
            }
        }

        // 6. %files section
        let files_ctx = FilesContext::inspect(workspace_path, Some(manifest), Some(meson));
        let files = generate_files_section(&files_ctx);
        spec.push_str(&files);

        if !files.ends_with("\n\n") {
            spec.push('\n');
        }

        // 7. %changelog section
        let (version, _) = HeaderSection::detect_version_and_prefix(workspace_path)
            .unwrap_or_else(|| ("0.1.0".to_string(), "".to_string()));
        let release = "1";

        let metainfo_xml =
            find_metainfo_file(workspace_path).and_then(|path| std::fs::read_to_string(path).ok());

        let changelog =
            ChangelogContext::new(cli.packager_name(), cli.packager_email(), &version, release)
                .generate(metainfo_xml.as_deref());

        spec.push_str(&changelog);
        if !spec.ends_with('\n') {
            spec.push('\n');
        }

        spec
    }
}

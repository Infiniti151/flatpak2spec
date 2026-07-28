// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Infiniti151

use crate::manifest::FlatpakManifest;
use crate::meson::MesonProject;
use crate::sections::build::BuildSection;
use crate::sections::deps::DepsSection;
use crate::sections::description::DescriptionSection;
use crate::sections::header::HeaderSection;
use crate::sections::scriptlets::ScriptletsSection;
use std::path::Path;

pub struct SpecGenerator;

impl SpecGenerator {
    pub fn generate(
        manifest: &FlatpakManifest,
        meson: &MesonProject,
        workspace_path: &Path,
        repo_url: &str,
        bug_url: Option<&str>,
    ) -> String {
        let mut spec = String::new();

        // 1. Preamble / Header block
        let header = HeaderSection::generate(manifest, meson, workspace_path, repo_url, bug_url);
        spec.push_str(&header);

        // 2. Dependencies (BuildRequires & Requires)
        let deps = DepsSection::generate(meson, workspace_path);
        spec.push_str(&deps);

        if !deps.ends_with("\n\n") {
            spec.push('\n');
        }

        // 3. %description (DescriptionSection handles its own header title)
        let description = DescriptionSection::generate(workspace_path, &header);
        spec.push_str(&description);
        spec.push('\n');

        // 4. Build stages (%prep, %build, %install, %check)
        let build = BuildSection::generate(meson, workspace_path);
        spec.push_str(&build);

        // 5. Scriptlets (%post, %postun, %posttrans)
        let scriptlets = ScriptletsSection::generate(meson);
        if !scriptlets.is_empty() {
            spec.push_str(&scriptlets);
            if !scriptlets.ends_with("\n\n") {
                spec.push('\n');
            }
        }

        spec
    }
}

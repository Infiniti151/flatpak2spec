%global         debug_package %{nil}

Name:           flatpak2spec
Version:        0.3.0
Release:        1%{?dist}
Summary:        CLI tool to generate Fedora-compliant RPM spec files from Flatpak repositories
License:        GPL-3.0-or-later
URL:            https://github.com/Infiniti151/%{name}
BugURL:         https://github.com/Infiniti151/%{name}/issues

Source0:        %{url}/releases/download/v%{version}/%{name}-%{version}-x86_64.tar.gz
Source1:        %{url}/releases/download/v%{version}/%{name}-%{version}-aarch64.tar.gz

ExclusiveArch:  x86_64 aarch64

%description
flatpak2spec is a lightweight CLI utility that inspects Flatpak repositories,
manifests, and associated project metadata to generate modern, clean,
and Fedora-compliant RPM spec files.

Features:
- Remote & Local Workspace Inspection: Supports direct workspace parsing from
  local filesystem directories or remote repositories on GitHub, GitLab,
  and Codeberg.
- Manifest & Metadata Parsing: Inspects Flatpak JSON/YAML manifests, AppStream
  metadata (.metainfo.xml), and Meson project configurations (meson.build).
- Changelog & Release Notes Extraction: Automatically parses project changelogs
  and release notes to populate the RPM %%changelog section cleanly.
- Smart Version & Forge Detection: Queries remote forge tags to determine the
  latest semantic release version, handles URL prefixes (v1.0 vs 1.0), and
  formats accurate %%forgesetup or Source0 download links.
- Automatic Submodule Handling: Detects .gitmodules within target repositories
  and emits Git fetching logic in %%prep to streamline online CI archive builds.
- Idiomatic Spec Output: Leverages modern Fedora packaging macros (%%meson,
  %%meson_build, %%find_lang) and %%check validation steps
  (desktop-file-validate, appstream-util).
- Copr & CI-Ready: Built specifically for automated online build environments
  such as Fedora Copr (with internet enabled), GitHub Actions, and mock chroots.
- Noarch Detection: Automatically detects asset-only or script projects to emit
  BuildArch: noarch when appropriate.

%prep
%ifarch x86_64
%autosetup -c -n %{name}-%{version} -T -a 0
%endif
%ifarch aarch64
%autosetup -c -n %{name}-%{version} -T -a 1
%endif

%build
# Pre-compiled binary release; no compilation required in build stage.

%install
install -D -m 0755 %{name} %{buildroot}%{_bindir}/%{name}

%check
%{buildroot}%{_bindir}/%{name} --help > /dev/null 2>&1

%files
%license LICENSE
%doc README.md
%{_bindir}/%{name}

%changelog
* Mon Aug 03 2026 Infiniti151 <43163551+Infiniti151@users.noreply.github.com> - 0.3.0-1
- Expand doc scanning targets to include AUTHORS, CONTRIBUTORS, and TODO
- Resolve external Module::Path and Source::Path files dynamically
- Sanitize app_id and resolve external path references
- Strip .devel and .Devel suffixes during App ID sanitization
- Perform exact stem and extension matching for docs and licenses
- Collapse nested if blocks to satisfy Clippy lints cleanly
- Add signature verification instructions
- Fix formatting
- Use git-cliff for automated changelog generation
- Add SSH asset signing for release packages
- ...see upstream for full release notes

* Sat Aug 01 2026 Infiniti151 <43163551+Infiniti151@users.noreply.github.com> - 0.2.0-1
- Support Git submodule resolution and tracking
- Improve forge integration via forge-srpm-macros
- Enhance Python environment detection

* Fri Jul 31 2026 Infiniti151 <43163551+Infiniti151@users.noreply.github.com> - 0.1.0-1
- Support remote and local repository parsing
- Parse Flatpak manifests, AppStream metadata, and Meson configurations
- Extract project changelogs to generate RPM %changelog entries
- Detect release versions and format forge download URLs dynamically
- Detect asset-only projects to assign BuildArch: noarch
- Generate idiomatic Fedora RPM spec files with standard macros
- Optimize output for Copr, GitHub Actions, and mock chroots
- Initial release


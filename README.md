# flatpak2spec
[![Build](https://img.shields.io/github/actions/workflow/status/Infiniti151/flatpak2spec/release.yml?branch=main&style=for-the-badge&logo=github-actions&logoColor=white&label=Build&color=%23007808)](https://github.com/Infiniti151/flatpak2spec/actions/workflows/release.yml) [![COPR Build Status](https://img.shields.io/badge/dynamic/json?url=https://copr.fedorainfracloud.org/api_3/build/list/%3Fownername%3Dinfiniti151%26projectname%3Dflatpak2spec%26packagename%3Dflatpak2spec%26limit%3D1&query=$.items[0].state&label=COPR&style=for-the-badge&logo=fedora&logoColor=white&color=%2351A2DA)](https://copr.fedorainfracloud.org/coprs/infiniti151/flatpak2spec/package/flatpak2spec/) [![Latest Release](https://img.shields.io/github/v/release/Infiniti151/flatpak2spec?style=for-the-badge&logo=github&color=blue)](https://github.com/Infiniti151/flatpak2spec/releases) [![Downloads](https://img.shields.io/github/downloads/Infiniti151/flatpak2spec/total.svg?style=for-the-badge&logo=github&color=orange)](https://github.com/Infiniti151/flatpak2spec/releases) [![License](https://img.shields.io/github/license/Infiniti151/flatpak2spec?style=for-the-badge&logo=spdx&logoColor=white&color=yellow&label=License)](https://github.com/Infiniti151/flatpak2spec/blob/main/LICENSE)

`flatpak2spec` is a fast, robust Rust CLI tool designed to inspect Flatpak application repositories (or remote URLs) and automatically generate production-ready, Fedora-compliant RPM .spec files following official Fedora Packaging Guidelines.

> [!IMPORTANT]
> **Build System Support:** `flatpak2spec` currently supports Flatpak manifests that utilize the **Meson** build system (`buildstream` / `meson` build options). Support for Cargo, CMake, and Autotools build systems is planned for future releases.

## ✨ Features

- 🚀 **Remote & Local Repositories:** Supports direct cloning and workspace parsing from GitHub, GitLab, Codeberg, or local filesystem directories.
- 📦 **Manifest & Metadata Parsing:** Inspects Flatpak JSON/YAML manifests, AppStream metadata (`.metainfo.xml`), and Meson project configurations (`meson.build`).
- 📝 **Changelog & Release Notes Extraction:** Automatically parses project changelogs and release notes to populate the RPM `%changelog` section cleanly.
- 🏷️ **Smart Version & Forge Detection:** Queries remote forge tags to determine the latest semantic release version, handles URL prefixes (`v1.0` vs `1.0`), and formats accurate `Source0` download links.
- 🛠️ **Fedora-Compliant Output:** Generates clean RPM spec files adhering to modern Fedora packaging standards, including standard macros (`%meson`, `%meson_build`, `%find_lang`) and `%check` validation steps (`desktop-file-validate`, `appstream-util`).
- ⚡ **Copr & CI-Ready:** Optimized out-of-the-box for online build environments such as Fedora Copr, GitHub Actions, and local `mock` chroots.
- 🎯 **Noarch Detection:** Automatically detects asset-only or script projects to emit `BuildArch: noarch` when appropriate.

## 📥 Installation

### 📦 Fedora / RHEL (Fedora Copr)

The easiest way to install and stay updated on Fedora or Enterprise Linux systems is via Copr:

**Enable the Copr repository**
```
sudo dnf copr enable infiniti151/flatpak2spec
```

**Install flatpak2spec**
```
sudo dnf install flatpak2spec
```

### 🚀 Pre-compiled Binaries (GitHub Releases)

If you prefer not to install via package manager or are running a non-RPM Linux distribution, grab the latest pre-compiled release binary for your architecture (x86_64 or aarch64):

**Extract the release tarball**
```
tar -xzf flatpak2spec-*-x86_64.tar.gz
```

**Make executable and move to /usr/local/bin**
```bash
chmod +x flatpak2spec
sudo mv flatpak2spec /usr/local/bin/
```

### 🛠️ Manual Build (From Source)

Ensure you have Rust and Cargo installed, then build directly from source:
```bash
git clone https://github.com/Infiniti151/flatpak2spec.git
cd flatpak2spec
cargo build --release
```

The compiled binary will be available at target/release/flatpak2spec.

## 💻 Usage

```
flatpak2spec [OPTIONS] --repo <PATH_OR_URL>

Options:
  -r, --repo <PATH_OR_URL>   Path or URL to the Flatpak application repository
  -b, --bug-url <URL>        Bug tracker URL for the BugURL spec field (optional)
  -p, --packager <PACKAGER>  Packager name for the %changelog section (optional)
  -e, --email <EMAIL>        Packager email for the %changelog section (optional)
  -o, --output <FILE>        Optional output file path (defaults to stdout if omitted)
  -v, --verbose...           Enable verbose logging output
  -h, --help                 Print help
  -V, --version              Print version
  ```

## 📄 Generated Spec File

Command:

```
flatpak2spec -b https://github.com/Infiniti151/flatpak-apps -p Infiniti151 -e 43163551+Infiniti151@users.noreply.github.com -r https://github.com/alainm23/planify
```

Output:
```
%global         app_id io.github.alainm23.planify.Devel

Name:           planify
Version:        4.19.5
Release:        1%{?dist}
Summary:        Forget about forgetting things
License:        GPL-3.0+
URL:            https://github.com/alainm23/planify
BugURL:         https://github.com/Infiniti151/flatpak-apps

Source0:        %{url}/archive/v%{version}.tar.gz

BuildRequires:  meson
BuildRequires:  ninja-build
BuildRequires:  desktop-file-utils
BuildRequires:  gcc
BuildRequires:  gettext
BuildRequires:  glibc-langpack-en
BuildRequires:  gtk-update-icon-cache
BuildRequires:  libappstream-glib
BuildRequires:  pkgconfig(chrono)
BuildRequires:  pkgconfig(gee-0.8)
BuildRequires:  pkgconfig(gio-2.0)
BuildRequires:  pkgconfig(glib-2.0)
BuildRequires:  pkgconfig(gtk4)
BuildRequires:  pkgconfig(gtksourceview-5)
BuildRequires:  pkgconfig(gxml-0.20)
BuildRequires:  pkgconfig(icu-uc)
BuildRequires:  pkgconfig(json-glib-1.0)
BuildRequires:  pkgconfig(libadwaita-1) >= 1.7.0
BuildRequires:  pkgconfig(libecal-2.0) >= 3.45.1
BuildRequires:  pkgconfig(libedataserver-1.2) >= 3.45.1
BuildRequires:  pkgconfig(libical-glib)
BuildRequires:  pkgconfig(libportal)
BuildRequires:  pkgconfig(libportal-gtk4)
BuildRequires:  pkgconfig(libsecret-1)
BuildRequires:  pkgconfig(libsoup-3.0)
BuildRequires:  pkgconfig(libspelling-1)
BuildRequires:  pkgconfig(sqlite3)
BuildRequires:  valac

%description
Planify is your modern and powerful task manager that helps you keep your life organized. With a clean and intuitive interface, cloud synchronization, and advanced features, you'll never forget what matters again.

✨ Core Features:
- Modern and clean interface designed with GTK4 and libadwaita
- Drag and drop to organize tasks and projects effortlessly
- Visual progress indicators for each project
- Smart organization with sections and custom labels
- Calendar integration to visualize your schedule
- Multiple reminders per task to never miss a deadline
- Dark mode with seamless system theme integration
- Quick and powerful search to find anything instantly
- Recurring tasks with flexible patterns
- Attachments and links in your tasks

☁️ Cloud Synchronization:
- Full synchronization with Todoist to access your tasks from anywhere
- Support for Nextcloud and CalDAV servers (Radicale, Baïkal) to keep your data private
- Offline mode: work without internet and sync when you're back online
- Cross-platform synchronization to access from any device

* Planify is not created by, affiliated with, or supported by Doist

%prep
%autosetup -C -p1

%build
%meson
%meson_build

%install
%meson_install
%find_lang %{name}

%check
%meson_test
desktop-file-validate %{buildroot}%{_datadir}/applications/*.desktop
appstream-util validate-relax --nonet %{buildroot}%{_metainfodir}/*.metainfo.xml
glib-compile-schemas --dry-run --strict %{buildroot}%{_datadir}/glib-2.0/schemas/

%files -f %{name}.lang
%license LICENSE
%doc README.md
%{_bindir}/%{app_id}
%{_datadir}/%{app_id}
%{_datadir}/applications/%{app_id}.desktop
%{_datadir}/icons/hicolor/*/apps/%{app_id}*
%{_datadir}/glib-2.0/schemas/*.gschema.xml
%{_datadir}/dbus-1/services/*.service
%{_metainfodir}/%{app_id}.metainfo.xml

%changelog
* Fri Jul 31 2026 Infiniti151 <43163551+Infiniti151@users.noreply.github.com> - 4.19.5-1
- Planify 4.19.5 is a maintenance release focused on bug fixes, reliability improvements, and new features.
- Bug Fixes:
- Fixed missed automatic backup on startup — if Planify was closed at midnight, the scheduled backup was silently skipped. Now, if automatic backup is enabled and no backup has run today, one is triggered immediately on startup.
- Fixed completing a recurring task from a notification — it was marking the task as fully completed instead of advancingto the next occurrence. Now correctly calls update_next_recurrency() as the UI does.
- Fixed keyboard navigation UX — Tab in EditableTextView now moves focus to the next widget instead of inserting spaces, and pressing Enter on a focused checkbox now completes the task.
- Fixed attachment file_size integer overflow — file_size is now treated as a string throughout, preventing unhandled overflow errors for values ≥ 2^63.
- Fixed drag and drop reliability and visual feedback in ReorderChild — softer background, subtle border, and rounder corners using the accent color.
- Fixed CalDAV related-to parsing — uses the already known related-id from libical and checks the reltype param to determine if a task is a parent.
- Fixed reading X-PINNED and X-APPLE-SORT-ORDER from CalDAV — replaced ICal.PropertyKind.from_string() with iteration over X_PROPERTY kind and matching by get_x_name(), fixing pin status and sort order persistence with Radicale and other servers.
- New Features:
- ... see upstream for full release notes
```

## 📂 Project Structure

```
flatpak2spec/
├── Cargo.toml                      # Project metadata, binary definition, and crate dependencies
├── .github/
│   └── workflows/
│       └── release.yml             # CI/CD workflow for automated building, attestation, and releases
└── src/
    ├── main.rs                     # Entry point: orchestrates CLI args, workspace cloning, and pipeline execution
    ├── spec.rs                     # Assembles all generated section strings into a final RPM .spec file
    ├── repository.rs               # Workspace preparation and remote Git repository cloning logic
    ├── forge.rs                    # Detects Git hosts (GitHub, GitLab, Codeberg) to format Source URLs
    ├── manifest.rs                 # Flatpak manifest parser and validation engine
    ├── meson.rs                    # Meson build file inspection and metadata extraction
    ├── utils.rs                    # Common filesystem, regex search, and string helper utilities
    └── sections/                   # Spec file section generators
        ├── mod.rs                  # Module aggregator exporting all spec section submodules
        ├── header.rs               # Generates RPM spec preamble metadata (Name, Version, License, URL, Source0)
        ├── deps.rs                 # Translates Flatpak modules/dependencies into BuildRequires and Requires directives
        ├── description.rs          # Generates the %description section
        ├── build.rs                # Generates %prep, %build, %install, and %check sections
        ├── scriptlets.rs           # Generates post/un scriptlets (icon cache updates, glib schemas)
        ├── files.rs                # Scans project artifacts to generate %files listings and %find_lang macros
        └── changelog.rs            # Generates the %changelog section
```

## 🤝 Contributing

Contributions, bug reports, and pull requests are welcome! Feel free to check out the issues page.

## 📜 License

This project is licensed under GPL-3.0-or-later.

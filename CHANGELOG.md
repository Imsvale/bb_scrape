# Changelog

## [Unreleased]

### Added
- **Build:** Cross-platform packaging scripts organized by platform
  - `scripts/windows/build.ps1` - Windows packaging (renamed from `zip.ps1`)
  - `scripts/mac-linux/build.sh` - Linux/macOS packaging with auto-detection of platform and architecture
  - `scripts/mac-linux/setup.sh` - Automated setup for non-developers
  - Supports x86_64 and aarch64 (Apple Silicon) architectures
  - All scripts flatten folder structure (binaries at zip root)

### Changed
- **Build:** Removed CLI feature gate - `cargo build --release` now builds both binaries
  - Simplified build process (no more `--features=cli` needed)
  - Removed unused `color-eyre` dependency (127 KB saved on CLI binary)
- **Release:** Updated packaging to support Windows, Linux, and macOS
  - Windows: `bb_scrape_v{version}_windows_x86_64.zip`
  - Linux: `bb_scrape_v{version}_linux_x86_64.zip`
  - macOS: `bb_scrape_v{version}_macos_x86_64.zip` / `macos_aarch64.zip`

## [1.3.1] - 2025-01-25

### Fixed
- Updated for changed site format.
- **Bug:** Progress counter suggesting success even on scrape failure.
  -  Now tracks success and failure separately.

### Added
- **UX:** Open output folder button.

### Changed
- **UX:** Changed default export format from CSV to TSV.
  - Makes more sense for copy & paste as it skips the "split data by delimiter" step.

## [1.3.0] - 2025-09-17

- New scrape: Injuries
- Column reordering (GUI only for now; not reflected in Copy/Export)
- Moved data table vertical scrollbar so it's outside the table
- Moved team list scrollbar to the edge of the panel
  - Was right next to, or even on top of team names
- Added horizontal scrollbar (when window is too small)
- Improved scrollbar visibility.
- Added season awareness (fetched from site and stored locally)
- Added some dev tests

# Known issues
- Team progress indicator is misleading on failure (e.g. site down)

## [1.2.0] - 2025-09-02

- **New scrape:** Game results.
- Various tweaks and improvements.

## [1.1.0] - 2025-08-26

Initial release. We don't talk about 1.0.0.
# Changelog
All notable changes to Gyr will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
## [v0.1.2] - 2022-09-13

### Added

* `-r`, `--replace` option to replace an existing Gyr instance.

### Changed

* Switched from dirty recursive directory walker to [walkdir](https://crates.io/crates/walkdir)

## [v0.1.1] - 2022-07-26

### Added

* VIM keybindings (`Ctrl+N`/`Ctrl+P`/`Ctrl+Y`)
* config: Disabling infinite scrolling via `hard_stop`

### Fixed

* ui: remove unused log
* Wait until loading finishes before showing the UI
* Switched to case insensitive sorting
* Read `$XDG_DATA_DIRS` instead of harcoded data paths

## [v0.1.0] - 2022-07-01

* Initial release

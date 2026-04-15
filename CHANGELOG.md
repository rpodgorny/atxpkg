# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [10.8.1] - 2026-04-15

### Changed
- Say "download" instead of "update" in downloadonly confirmation prompt.
- Update dependencies (rand 0.9.4).

### Security
- Update rustls-webpki to 0.103.12 to fix RUSTSEC-2026-0098.
- Update fastrand to 2.4.1 (previous version was yanked).

## [10.8.0] - 2026-04-06

### Added
- Add osv-scanner to audit.
- Add regular security check.

### Changed
- Update Rust version and edition.
- Update dependencies to make audit clean (rustls-webpki, quinn-proto, bytes and others).

## [10.7.0] - 2025-09-13

### Added
- Add experimental `upstall` command.

### Changed
- Pre-allocate memory in some places for a slight speedup.
- Improve repository reading error handling.
- Update dependencies.

## [10.6.0] - 2025-07-31

### Changed
- Pin Rust version and make cargo MSRV-aware.
- Better error handling.
- Update dependencies (tokio, openssl, zip, ring and others).

## [10.5.0] - 2024-12-29

### Added
- Add progress bar for local directory listings.

### Changed
- Drop dependency on `rayon` in favor of `scoped_threadpool`.
- Update dependencies (hashbrown and others).

## [10.4.0] - 2024-10-11

### Changed
- More sorting improvements.
- Update dependencies.

## [10.3.0] - 2024-10-01

### Changed
- Sort output of `list_installed` commands.
- Improvements to error handling and error messages.

## [10.2.0] - 2024-09-25

### Changed
- Error handling improvements in update code.

### Fixed
- Fix `--no` flag not being handled correctly.

## [10.1.0] - 2024-09-20

### Changed
- Reduce binary size.

## [10.0.0] - 2024-09-20

### Added
- Support for `unverified_ssl`.

### Changed
- Migrate HTTP client to `reqwest`.
- Use buffered reader when reading `installed.json`.
- Use buffered writer when saving installed packages and when unzipping.
- Flush buffered writer after download.
- Better return values from install/update/remove operations.
- Use mutable `installed_packages` so that the potentially modified package set can be persisted even on error.
- Compile with debug symbols and line numbers for better tracebacks.
- Improved logging and log levels.
- Minor progress bar fixes and fixes for progress bar issues over SSH connections to cygwin.
- Update dependencies.

---

**Note:** This changelog starts with the 10.x series (first Rust releases). Earlier Python-based versions (up to 4.3) are not included.

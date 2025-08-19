# atxpkg

A brain-dead simple package manager for Windows (and Linux).

Written in Rust for performance and reliability.

## Overview

atxpkg is a lightweight package manager designed for simplicity and ease of use. It manages packages in ZIP format and provides basic package operations like install, update, remove, and list functionality.

## Features

- **Simple package management**: Install, update, remove packages
- **Cross-platform**: Works on Windows and Linux
- **ZIP-based packages**: Uses standard ZIP format for packages
- **Offline support**: Can work without internet connection
- **Force operations**: Override existing files when needed
- **Download-only mode**: Fetch packages without installing
- **Package verification**: MD5 checksum validation
- **Untracked file detection**: Find files not managed by any package

## Installation

### From Source

Requires Rust 1.77 or later:

```bash
git clone <repository-url>
cd atxpkg
cargo build --release
```

The binary will be available at `target/release/atxpkg`.

## Usage

### Basic Commands

```bash
# Install packages
atxpkg install <package1> <package2>

# Update packages
atxpkg update <package1> <package2>

# Install or update (upstall - install if not present, update if installed)
atxpkg upstall <package1> <package2>

# Remove packages
atxpkg remove <package1> <package2>

# List available packages
atxpkg list_available

# List installed packages
atxpkg list_installed

# Check package integrity
atxpkg check <package1> <package2>

# Show untracked files
atxpkg show_untracked <path1> <path2>

# Clean download cache
atxpkg clean_cache
```

### Command Options

#### Global Options

- `--prefix <path>`: Set installation prefix (default: `/` on Linux, `c:/` on Windows)
- `--debug`: Enable debug logging

#### Install/Update/Upstall/Remove Options

- `-f, --force`: Force operation, overwrite existing files
- `-w, --downloadonly`: Only download packages, don't install
- `-y, --yes`: Automatically answer yes to all questions
- `-n, --no`: Automatically answer no to all questions
- `--offline`: Don't connect to online repositories
- `--if-installed <package>`: Only perform operation if specified package is installed
- `--unverified-ssl`: Don't verify SSL certificate validity

#### List Available Options

- `--offline`: Don't connect to online repositories
- `--unverified-ssl`: Don't verify SSL certificate validity

### Examples

```bash
# Install a package with force flag
atxpkg install --force mypackage

# Download packages without installing
atxpkg install --downloadonly package1 package2

# Update all installed packages (when no packages specified)
atxpkg update

# Install with custom prefix
atxpkg --prefix /opt install mypackage

# Work offline with local repository
atxpkg --offline list_available

# Check integrity of specific packages
atxpkg check package1 package2

# Find untracked files in a directory
atxpkg show_untracked /usr/local/bin

# Clean download cache
atxpkg clean_cache
```

## Package Format

atxpkg uses ZIP files with a specific naming convention:
- Format: `packagename-version-release.atxpkg.zip`
- Example: `myapp-1.2.3-1.atxpkg.zip`

## Development

### Testing

```bash
# Run tests
just test
# or
cargo nextest run
```

### Mutation Testing

```bash
just mutants
```

### Update Dependencies

```bash
just updeps
```

## License

PROPRIETARY - See license file for details.

## Author

Radek Podgorny <radek@podgorny.cz>

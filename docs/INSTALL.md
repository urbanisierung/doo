# Installation Scripts

This directory contains installation scripts for the Doo CLI tool that automatically download and install the latest release.

## Quick Installation

### Linux & macOS

```bash
curl -fsSL https://raw.githubusercontent.com/urbanisierung/clap/main/install.sh | bash
```

### Windows (PowerShell)

```powershell
iwr -useb https://raw.githubusercontent.com/urbanisierung/clap/main/install.ps1 | iex
```

## What the Scripts Do

### Unix Shell Script (`install.sh`)

1. **Platform Detection**: Automatically detects your OS (Linux/macOS) and architecture (x86_64/aarch64)
2. **Latest Release**: Fetches the latest release information from GitHub API
3. **Download**: Downloads the appropriate binary for your platform
4. **Installation**: Installs to an appropriate directory:
   - `/usr/local/bin` (if writable or with sudo)
   - `~/.local/bin` (fallback for user installation)
5. **PATH Management**: Ensures the installation directory is in your PATH
6. **Verification**: Tests that the installation was successful

### PowerShell Script (`install.ps1`)

1. **Platform Detection**: Detects Windows architecture (x86_64/aarch64)
2. **Latest Release**: Fetches the latest release from GitHub API
3. **Download**: Downloads the Windows executable
4. **Installation**: Installs to:
   - `%LOCALAPPDATA%\Programs\doo`
   - `%PROGRAMFILES%\doo` (if admin)
   - `%USERPROFILE%\.local\bin` (fallback)
5. **PATH Management**: Updates user PATH environment variable
6. **Verification**: Tests the installation

## Supported Platforms

The installation scripts support the following platforms that match the GitHub Actions release targets:

### Linux

- `x86_64` (Intel/AMD 64-bit) - `doo-linux-x86_64`
- `x86_64-musl` (Intel/AMD 64-bit, static) - `doo-linux-x86_64-musl`

### macOS

- `x86_64` (Intel Macs) - `doo-macos-x86_64`
- `aarch64` (Apple Silicon Macs) - `doo-macos-aarch64`

### Windows

- `x86_64` (Intel/AMD 64-bit) - `doo-windows-x86_64.exe`

**Note**: ARM64 builds for Linux and Windows are not currently available in automated releases.

## Manual Installation

If the automatic scripts don't work for your system:

1. Go to [GitHub Releases](https://github.com/urbanisierung/clap/releases)
2. Download the binary for your platform
3. Extract it and move to a directory in your PATH
4. Make it executable (Linux/macOS): `chmod +x doo`

## Troubleshooting

### Linux/macOS Issues

**Permission Denied**

```bash
# If installation fails due to permissions, try:
curl -fsSL https://raw.githubusercontent.com/urbanisierung/clap/main/install.sh | bash
# Then add ~/.local/bin to your PATH if not already there
export PATH="$PATH:$HOME/.local/bin"
```

**Binary Not Found After Installation**

```bash
# Check if the installation directory is in PATH
echo $PATH
# Add it manually if needed
echo 'export PATH="$PATH:$HOME/.local/bin"' >> ~/.bashrc
source ~/.bashrc
```

### Windows Issues

**Execution Policy**

```powershell
# If you get execution policy errors:
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
iwr -useb https://raw.githubusercontent.com/urbanisierung/clap/main/install.ps1 | iex
```

**PATH Not Updated**

```powershell
# Restart your command prompt or PowerShell session
# Or manually add to PATH:
$env:PATH += ";$env:LOCALAPPDATA\Programs\doo"
```

## Security

Both scripts:

- Download only from official GitHub releases
- Use HTTPS for all downloads
- Verify downloads before installation
- Don't execute arbitrary code from the internet beyond the installation script itself

The scripts are designed to be transparent and safe. You can always inspect them before running:

```bash
# Review the script before running
curl -fsSL https://raw.githubusercontent.com/urbanisierung/clap/main/install.sh
```

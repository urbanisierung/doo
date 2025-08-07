#!/bin/bash

# Doo CLI Installation Script
# Usage: curl -fsSL https://raw.githubusercontent.com/urbanisierung/doo/main/install.sh | bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
REPO_OWNER="urbanisierung"
REPO_NAME="doo"
BINARY_NAME="doo"
GITHUB_API="https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}"

# Helper functions
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Detect OS and architecture
detect_platform() {
    local os=$(uname -s | tr '[:upper:]' '[:lower:]')
    local arch=$(uname -m)
    
    case "$os" in
        linux*)
            OS="linux"
            ;;
        darwin*)
            OS="macos"
            ;;
        *)
            print_error "Unsupported operating system: $os"
            print_info "This script supports Linux and macOS only."
            print_info "For Windows, please use: iwr -useb https://raw.githubusercontent.com/${REPO_OWNER}/${REPO_NAME}/main/install.ps1 | iex"
            exit 1
            ;;
    esac
    
    case "$arch" in
        x86_64|amd64)
            ARCH="x86_64"
            ;;
        aarch64|arm64)
            ARCH="aarch64"
            ;;
        *)
            print_error "Unsupported architecture: $arch"
            print_info "Supported architectures: x86_64, aarch64"
            exit 1
            ;;
    esac
    
    if [[ "$OS" == "macos" && "$ARCH" == "aarch64" ]]; then
        TARGET="${BINARY_NAME}-macos-aarch64"
    elif [[ "$OS" == "macos" && "$ARCH" == "x86_64" ]]; then
        TARGET="${BINARY_NAME}-macos-x86_64"
    elif [[ "$OS" == "linux" && "$ARCH" == "x86_64" ]]; then
        TARGET="${BINARY_NAME}-linux-x86_64"
    elif [[ "$OS" == "linux" && "$ARCH" == "aarch64" ]]; then
        # Note: Currently only x86_64 Linux is built in CI
        print_error "ARM64 Linux builds are not currently available"
        print_info "Available builds: linux-x86_64, macos-x86_64, macos-aarch64, windows-x86_64"
        exit 1
    else
        print_error "No binary available for ${OS}-${ARCH}"
        exit 1
    fi
    
    print_info "Detected platform: ${OS}-${ARCH}"
    print_info "Target binary: ${TARGET}"
}

# Check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Get latest release info
get_latest_release() {
    print_info "Fetching latest release information..."
    
    if command_exists curl; then
        RELEASE_DATA=$(curl -fsSL "${GITHUB_API}/releases/latest" 2>/dev/null)
    elif command_exists wget; then
        RELEASE_DATA=$(wget -qO- "${GITHUB_API}/releases/latest" 2>/dev/null)
    else
        print_error "Neither curl nor wget is available"
        print_info "Please install curl or wget to continue"
        exit 1
    fi
    
    if [[ -z "$RELEASE_DATA" ]]; then
        print_error "Failed to fetch release information from GitHub"
        print_info "This could be due to:"
        print_info "  • Network connectivity issues"
        print_info "  • GitHub API rate limiting"
        print_info "  • Repository not found"
        exit 1
    fi
    
    # Extract tag name (version)
    VERSION=$(echo "$RELEASE_DATA" | grep '"tag_name":' | sed -E 's/.*"tag_name": "([^"]+)".*/\1/')
    
    if [[ -z "$VERSION" ]]; then
        print_error "Failed to parse release version from GitHub response"
        print_info "Please try again later or install manually"
        exit 1
    fi
    
    print_info "Latest version: $VERSION"
    
    # Construct download URL
    DOWNLOAD_URL="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/${VERSION}/${TARGET}"
    print_info "Download URL: $DOWNLOAD_URL"
}

# Download binary
download_binary() {
    print_info "Downloading ${BINARY_NAME} ${VERSION}..."
    
    # Create temporary directory
    TMP_DIR=$(mktemp -d)
    TMP_FILE="${TMP_DIR}/${BINARY_NAME}"
    
    # Download the binary
    if command_exists curl; then
        if ! curl -fsSL -o "$TMP_FILE" "$DOWNLOAD_URL"; then
            print_error "Failed to download binary"
            rm -rf "$TMP_DIR"
            exit 1
        fi
    elif command_exists wget; then
        if ! wget -q -O "$TMP_FILE" "$DOWNLOAD_URL"; then
            print_error "Failed to download binary"
            rm -rf "$TMP_DIR"
            exit 1
        fi
    fi
    
    # Make binary executable
    chmod +x "$TMP_FILE"
    
    print_success "Binary downloaded successfully"
}

# Determine installation directory
get_install_dir() {
    # Check if user has write permission to /usr/local/bin
    if [[ -w "/usr/local/bin" ]] || [[ "$(id -u)" -eq 0 ]]; then
        INSTALL_DIR="/usr/local/bin"
        NEEDS_SUDO=false
    # Check if /usr/local/bin exists and we can sudo
    elif [[ -d "/usr/local/bin" ]] && command_exists sudo; then
        INSTALL_DIR="/usr/local/bin"
        NEEDS_SUDO=true
    # Fall back to user's local bin directory
    else
        INSTALL_DIR="$HOME/.local/bin"
        NEEDS_SUDO=false
        
        # Create directory if it doesn't exist
        mkdir -p "$INSTALL_DIR"
        
        # Check if it's in PATH
        if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
            print_warning "$INSTALL_DIR is not in your PATH"
            print_info "Add it to your PATH by adding this line to your shell profile:"
            print_info "  export PATH=\"\$PATH:$INSTALL_DIR\""
        fi
    fi
    
    print_info "Install directory: $INSTALL_DIR"
}

# Install binary
install_binary() {
    local target_path="${INSTALL_DIR}/${BINARY_NAME}"
    
    print_info "Installing ${BINARY_NAME} to ${target_path}..."
    
    if [[ "$NEEDS_SUDO" == "true" ]]; then
        if ! sudo cp "$TMP_FILE" "$target_path"; then
            print_error "Failed to install binary (sudo required)"
            rm -rf "$TMP_DIR"
            exit 1
        fi
        sudo chmod +x "$target_path"
    else
        if ! cp "$TMP_FILE" "$target_path"; then
            print_error "Failed to install binary"
            rm -rf "$TMP_DIR"
            exit 1
        fi
        chmod +x "$target_path"
    fi
    
    # Clean up
    rm -rf "$TMP_DIR"
    
    print_success "${BINARY_NAME} ${VERSION} installed successfully!"
}

# Verify installation
verify_installation() {
    print_info "Verifying installation..."
    
    if command_exists "$BINARY_NAME"; then
        local installed_version
        installed_version=$("$BINARY_NAME" --version 2>/dev/null | head -n1 || echo "unknown")
        print_success "✓ ${BINARY_NAME} is available in PATH"
        print_info "Installed version: $installed_version"
    else
        print_warning "✗ ${BINARY_NAME} is not available in PATH"
        print_info "You may need to restart your shell or update your PATH"
        print_info "Try running: export PATH=\"\$PATH:${INSTALL_DIR}\""
    fi
}

# Show usage examples
show_usage() {
    echo ""
    print_success "🎉 Installation complete!"
    echo ""
    print_info "Get started with ${BINARY_NAME}:"
    echo "  ${BINARY_NAME} --help                    # Show help"
    echo "  ${BINARY_NAME} import owner/repo        # Import config from GitHub"
    echo "  ${BINARY_NAME} sync                     # Sync imported configs"
    echo "  ${BINARY_NAME}                          # Interactive mode"
    echo ""
    print_info "For more information, visit: https://github.com/${REPO_OWNER}/${REPO_NAME}"
}

# Main installation flow
main() {
    echo "🚀 Doo CLI Installation Script"
    echo "==============================="
    echo ""
    
    detect_platform
    get_latest_release
    download_binary
    get_install_dir
    install_binary
    verify_installation
    show_usage
}

# Run main function
main "$@"

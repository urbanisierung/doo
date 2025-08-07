# Doo CLI Installation Script for Windows
# Usage: iwr -useb https://raw.githubusercontent.com/urbanisierung/doo/main/install.ps1 | iex

param(
    [string]$InstallDir = "",
    [switch]$Force
)

# Configuration
$RepoOwner = "urbanisierung"
$RepoName = "clap"
$BinaryName = "doo"
$GitHubAPI = "https://api.github.com/repos/$RepoOwner/$RepoName"

# Color output functions
function Write-ColorOutput {
    param(
        [string]$Message,
        [string]$Color = "White"
    )
    
    $colors = @{
        "Red" = [ConsoleColor]::Red
        "Green" = [ConsoleColor]::Green
        "Yellow" = [ConsoleColor]::Yellow
        "Blue" = [ConsoleColor]::Blue
        "White" = [ConsoleColor]::White
        "Cyan" = [ConsoleColor]::Cyan
    }
    
    Write-Host $Message -ForegroundColor $colors[$Color]
}

function Write-Info {
    param([string]$Message)
    Write-ColorOutput "[INFO] $Message" "Blue"
}

function Write-Success {
    param([string]$Message)
    Write-ColorOutput "[SUCCESS] $Message" "Green"
}

function Write-Warning {
    param([string]$Message)
    Write-ColorOutput "[WARNING] $Message" "Yellow"
}

function Write-Error {
    param([string]$Message)
    Write-ColorOutput "[ERROR] $Message" "Red"
}

# Detect platform and architecture
function Get-Platform {
    $arch = (Get-WmiObject Win32_Processor).Architecture
    
    switch ($arch) {
        9 { $architecture = "x86_64" }  # AMD64
        12 { 
            Write-Error "ARM64 Windows builds are not currently available"
            Write-Info "Available builds: windows-x86_64, linux-x86_64, macos-x86_64, macos-aarch64"
            exit 1
        } # ARM64
        default {
            Write-Error "Unsupported architecture: $arch"
            Write-Info "Supported architectures: x86_64 (AMD64)"
            exit 1
        }
    }
    
    $target = "$BinaryName-windows-$architecture.exe"
    
    Write-Info "Detected platform: Windows-$architecture"
    Write-Info "Target binary: $target"
    
    return $target
}

# Get latest release information
function Get-LatestRelease {
    Write-Info "Fetching latest release information..."
    
    try {
        $releaseData = Invoke-RestMethod -Uri "$GitHubAPI/releases/latest" -Method Get
        $version = $releaseData.tag_name
        
        if (-not $version) {
            Write-Error "Failed to get latest release version"
            exit 1
        }
        
        Write-Info "Latest version: $version"
        return $version
    }
    catch {
        Write-Error "Failed to fetch release information: $_"
        exit 1
    }
}

# Download binary
function Download-Binary {
    param(
        [string]$Version,
        [string]$Target
    )
    
    $downloadUrl = "https://github.com/$RepoOwner/$RepoName/releases/download/$Version/$Target"
    $tempDir = [System.IO.Path]::GetTempPath()
    $tempFile = Join-Path $tempDir "$BinaryName.exe"
    
    Write-Info "Downloading $BinaryName $Version..."
    Write-Info "Download URL: $downloadUrl"
    
    try {
        # Use TLS 1.2
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        
        Invoke-WebRequest -Uri $downloadUrl -OutFile $tempFile
        Write-Success "Binary downloaded successfully"
        return $tempFile
    }
    catch {
        Write-Error "Failed to download binary: $_"
        exit 1
    }
}

# Determine installation directory
function Get-InstallDirectory {
    param([string]$PreferredDir)
    
    if ($PreferredDir) {
        if (Test-Path $PreferredDir) {
            return $PreferredDir
        } else {
            Write-Warning "Specified directory does not exist: $PreferredDir"
        }
    }
    
    # Try common installation directories
    $possibleDirs = @(
        "$env:LOCALAPPDATA\Programs\$BinaryName",
        "$env:PROGRAMFILES\$BinaryName",
        "$env:USERPROFILE\.local\bin"
    )
    
    foreach ($dir in $possibleDirs) {
        try {
            if (-not (Test-Path $dir)) {
                New-Item -ItemType Directory -Path $dir -Force | Out-Null
            }
            
            # Test write permission
            $testFile = Join-Path $dir "test_write_permission.tmp"
            "" | Out-File -FilePath $testFile -Force
            Remove-Item $testFile -Force
            
            Write-Info "Install directory: $dir"
            return $dir
        }
        catch {
            Write-Warning "Cannot use directory $dir`: $_"
            continue
        }
    }
    
    Write-Error "Could not find a suitable installation directory"
    exit 1
}

# Install binary
function Install-Binary {
    param(
        [string]$TempFile,
        [string]$InstallDirectory
    )
    
    $targetPath = Join-Path $InstallDirectory "$BinaryName.exe"
    
    Write-Info "Installing $BinaryName to $targetPath..."
    
    try {
        if (Test-Path $targetPath) {
            if (-not $Force) {
                $response = Read-Host "$BinaryName already exists. Overwrite? (y/N)"
                if ($response -notmatch "^[Yy]") {
                    Write-Info "Installation cancelled by user"
                    exit 0
                }
            }
        }
        
        Copy-Item $TempFile $targetPath -Force
        Remove-Item $TempFile -Force
        
        Write-Success "$BinaryName installed successfully!"
        return $targetPath
    }
    catch {
        Write-Error "Failed to install binary: $_"
        exit 1
    }
}

# Update PATH environment variable
function Update-PathEnvironment {
    param([string]$InstallDirectory)
    
    $currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
    
    if ($currentPath -split ";" -contains $InstallDirectory) {
        Write-Info "Installation directory is already in PATH"
        return
    }
    
    Write-Info "Adding installation directory to PATH..."
    
    try {
        $newPath = if ($currentPath) { "$currentPath;$InstallDirectory" } else { $InstallDirectory }
        [Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
        
        # Update current session PATH
        $env:PATH += ";$InstallDirectory"
        
        Write-Success "PATH updated successfully"
        Write-Warning "You may need to restart your command prompt for PATH changes to take effect"
    }
    catch {
        Write-Warning "Failed to update PATH: $_"
        Write-Info "You can manually add '$InstallDirectory' to your PATH environment variable"
    }
}

# Verify installation
function Test-Installation {
    param([string]$BinaryPath)
    
    Write-Info "Verifying installation..."
    
    try {
        $version = & $BinaryPath --version 2>$null
        if ($LASTEXITCODE -eq 0) {
            Write-Success "✓ $BinaryName is working correctly"
            Write-Info "Installed version: $version"
        } else {
            Write-Warning "✗ $BinaryName may not be working correctly"
        }
    }
    catch {
        Write-Warning "✗ Could not verify installation: $_"
    }
    
    # Test if it's available in PATH
    try {
        $pathVersion = & $BinaryName --version 2>$null
        if ($LASTEXITCODE -eq 0) {
            Write-Success "✓ $BinaryName is available in PATH"
        } else {
            Write-Warning "✗ $BinaryName is not available in PATH"
        }
    }
    catch {
        Write-Warning "✗ $BinaryName is not available in PATH"
        Write-Info "You may need to restart your command prompt"
    }
}

# Show usage examples
function Show-Usage {
    Write-Host ""
    Write-Success "🎉 Installation complete!"
    Write-Host ""
    Write-Info "Get started with $BinaryName`:"
    Write-Host "  $BinaryName --help                    # Show help"
    Write-Host "  $BinaryName import owner/repo        # Import config from GitHub"
    Write-Host "  $BinaryName sync                     # Sync imported configs"
    Write-Host "  $BinaryName                          # Interactive mode"
    Write-Host ""
    Write-Info "For more information, visit: https://github.com/$RepoOwner/$RepoName"
}

# Main installation function
function Install-DooCLI {
    Write-Host "🚀 Doo CLI Installation Script for Windows" -ForegroundColor Cyan
    Write-Host "===========================================" -ForegroundColor Cyan
    Write-Host ""
    
    # Check PowerShell version
    if ($PSVersionTable.PSVersion.Major -lt 3) {
        Write-Error "PowerShell 3.0 or later is required"
        exit 1
    }
    
    $target = Get-Platform
    $version = Get-LatestRelease
    $tempFile = Download-Binary -Version $version -Target $target
    $installDir = Get-InstallDirectory -PreferredDir $InstallDir
    $binaryPath = Install-Binary -TempFile $tempFile -InstallDirectory $installDir
    Update-PathEnvironment -InstallDirectory $installDir
    Test-Installation -BinaryPath $binaryPath
    Show-Usage
}

# Error handling
trap {
    Write-Error "An unexpected error occurred: $_"
    exit 1
}

# Run the installation
Install-DooCLI

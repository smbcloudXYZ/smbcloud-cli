$ErrorActionPreference = "Stop"

# =============================================================================
# Define base variables
# =============================================================================

$name = "smbcloud-cli"
$binary="$name.exe"
$version="0.3.26"
$githubRepo="https://github.com/smbcloudXYZ/smbcloud-cli"
$downloadBaseUrl="https://github.com/$githubRepo/releases/download/$version"

if ($version -eq "latest") {
  # The latest version is accessible from a slightly different URL
  $downloadBaseUrl="https://github.com/$githubRepo/releases/latest/download"
}

# =============================================================================
# Determine system architecture and obtain the relevant binary to download
# - you can add more "if" conditions to support additional architectures
# =============================================================================

$type = (Get-ComputerInfo).CsSystemType.ToLower()
if ($type.StartsWith("x64")) {
    $downloadFile = "smb"
} else {
    Write-Host "[Error]" -ForegroundColor Red
    Write-Host "Unsupported Archi<tecture: $type" -ForegroundColor Red
    [Environment]::Exit(1)
}

# =============================================================================
# Create installation directory
# =============================================================================

$destDir = "$env:USERPROFILE\AppData\Local\$name"
$destBin = "$destDir\$binary"
Write-Host "Creating Install Directory" -ForegroundColor White
Write-Host " $destDir"

# Create the directory if it doesn't exist
if (-Not (Test-Path $destDir)) {
    New-Item -ItemType Directory -Path $destDir
}

# =============================================================================
# Download the binary to the installation directory
# =============================================================================

$downloadUrl = "$downloadBaseUrl/$downloadFile"
Write-Host "Downloading Binary" -ForegroundColor White
Write-Host " From: $downloadUrl"
Write-Host " Path: $destBin"
Invoke-WebRequest -Uri $downloadUrl -OutFile "$destBin"

# =============================================================================
# Add installation directory to the user's PATH if not present
# =============================================================================

$currentPath = [System.Environment]::GetEnvironmentVariable('Path', [System.EnvironmentVariableTarget]::User)
if (-Not ($currentPath -like "*$destDir*")) {
    Write-Host "Adding Install Directory To System Path" -ForegroundColor White
    Write-Host " $destBin"
    [System.Environment]::SetEnvironmentVariable('Path', "$currentPath;$destDir", [System.EnvironmentVariableTarget]::User)
}

# =============================================================================
# Display post installation message
# =============================================================================

Write-Host "Installation Complete" -ForegroundColor Green
Write-Host " Restart your shell to starting using '$binary'. Run '$binary --help' for more information"

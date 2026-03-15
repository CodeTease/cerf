$ErrorActionPreference = "Stop"

$BIN_NAME = "{{bin}}"
$VERSION = "{{version}}"
$REPO = "{{repository}}"
$INSTALL_PATH = "{{path-win}}"

Write-Host "Installing $BIN_NAME v$VERSION..." -ForegroundColor Cyan

# Expand environment variables if any
$ExpandedPath = [System.Environment]::ExpandEnvironmentVariables($INSTALL_PATH)
if (-not (Test-Path -Path $ExpandedPath)) {
    New-Item -ItemType Directory -Force -Path $ExpandedPath | Out-Null
}

$ARCH = $env:PROCESSOR_ARCHITECTURE
$TARGET = ""

[IF target.x86_64-pc-windows-msvc]
if ($ARCH -eq "AMD64") { $TARGET = "x86_64-pc-windows-msvc" }
[/IF]
[IF target.aarch64-pc-windows-msvc]
if ($ARCH -eq "ARM64") { $TARGET = "aarch64-pc-windows-msvc" }
[/IF]
[IF target.i686-pc-windows-msvc]
if ($ARCH -eq "x86") { $TARGET = "i686-pc-windows-msvc" }
[/IF]

if ([string]::IsNullOrEmpty($TARGET)) {
    Write-Host "Unsupported architecture: $ARCH" -ForegroundColor Red
    exit 1
}

$ASSET = "$BIN_NAME-$VERSION-$TARGET.zip"
$DOWNLOAD_URL = "$REPO/releases/download/v$VERSION/$ASSET"
$TMP_DIR = Join-Path $env:TEMP "$BIN_NAME-tmp"

if (Test-Path -Path $TMP_DIR) {
    Remove-Item -Recurse -Force -Path $TMP_DIR
}
New-Item -ItemType Directory -Force -Path $TMP_DIR | Out-Null

$ZIP_PATH = Join-Path $TMP_DIR $ASSET

Write-Host "Downloading $DOWNLOAD_URL..."
Invoke-WebRequest -Uri $DOWNLOAD_URL -OutFile $ZIP_PATH

Write-Host "Extracting..."
Expand-Archive -Path $ZIP_PATH -DestinationPath $TMP_DIR -Force

$EXTRACTED_BIN = Join-Path $TMP_DIR "$BIN_NAME.exe"
if (Test-Path -Path $EXTRACTED_BIN) {
    Copy-Item -Path $EXTRACTED_BIN -Destination $ExpandedPath -Force
} else {
    Write-Host "Binary not found in archive" -ForegroundColor Red
    Remove-Item -Recurse -Force -Path $TMP_DIR
    exit 1
}

Remove-Item -Recurse -Force -Path $TMP_DIR

Write-Host "$BIN_NAME installed successfully to $ExpandedPath!" -ForegroundColor Green
Write-Host "Please make sure $ExpandedPath is in your PATH." -ForegroundColor Yellow

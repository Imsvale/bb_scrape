# scripts/windows/build.ps1
param(
  [string]$OutDir = "artifacts",
  [string]$CargoTomlPath = "Cargo.toml",
  [string]$Arch = "windows_x86_64",
  [string[]]$ExtraFiles = @(),
  [switch]$BuildOnly,
  [switch]$PackageOnly,
  [switch]$UsePsZip
)

$ErrorActionPreference = "Stop"

# Validate conflicting options
if ($BuildOnly -and $PackageOnly) {
  Write-Host "Error: -BuildOnly and -PackageOnly are mutually exclusive" -ForegroundColor Red
  exit 1
}

function Get-VersionFromCargoToml([string]$path) {
  if (!(Test-Path -LiteralPath $path)) { throw "Cargo.toml not found at '$path'" }
  $lines = Get-Content -Raw -LiteralPath $path -Encoding UTF8
  $inPkg = $false
  foreach ($ln in ($lines -split "`r?`n")) {
    if ($ln -match '^\s*\[package\]') { $inPkg = $true; continue }
    if ($inPkg -and $ln -match '^\s*\[') { break }
    if ($inPkg -and $ln -match '^\s*version\s*=\s*"([^"]+)"') { return $Matches[1] }
  }
  throw "Could not find version in [package] section of $path"
}

function Ensure-Dir([string]$dir) {
  if (!(Test-Path -LiteralPath $dir)) { [void](New-Item -ItemType Directory -Force -Path $dir) }
}

function Get-SevenZipPath() {
  try { return (Get-Command 7z -ErrorAction Stop).Path } catch {
    $candidates = @(
      (Join-Path $Env:ProgramFiles '7-Zip\7z.exe'),
      (Join-Path ${Env:ProgramFiles(x86)} '7-Zip\7z.exe')
    )
    foreach ($p in $candidates) { if (Test-Path -LiteralPath $p) { return $p } }
    return $null
  }
}

try {
  $repoRoot = (& git rev-parse --show-toplevel).Trim()
  if ($repoRoot) { Set-Location $repoRoot }
} catch { }

# Build release binaries (default behavior unless -PackageOnly is specified)
if (-not $PackageOnly) {
  Write-Host "Building release binaries..."
  cargo build --release | Out-Host

  # If -BuildOnly, exit after building
  if ($BuildOnly) {
    Write-Host ""
    Write-Host "Build complete!" -ForegroundColor Green
    Write-Host "Binaries created:"
    Write-Host "  GUI: target\release\bb_scrape.exe"
    Write-Host "  CLI: target\release\cli.exe"
    exit 0
  }
}

$baseFiles = @(
  "target\release\bb_scrape.exe",
  "target\release\cli.exe",
  "README.md",
  "LICENSE"
)

$files = @()
$files += $baseFiles
if ($ExtraFiles) { $files += $ExtraFiles }

foreach ($f in $files) {
  if (!(Test-Path -LiteralPath $f)) {
    Write-Host "Error: File not found: $f" -ForegroundColor Red
    Write-Host ""
    if ($PackageOnly) {
      Write-Host "The -PackageOnly flag was used, but binaries don't exist." -ForegroundColor Yellow
      Write-Host "Please build first:"
      Write-Host "  cargo build --release"
      Write-Host "Or run without -PackageOnly to build automatically:"
      Write-Host "  .\scripts\windows\build.ps1"
    } else {
      Write-Host "Build failed or binaries were not created." -ForegroundColor Yellow
      Write-Host "Please check the build output above for errors."
    }
    exit 1
  }
}

$version = Get-VersionFromCargoToml -path $CargoTomlPath

# NEW: put versioned subdir under OutDir
$verDir = Join-Path $OutDir "v$version"
Ensure-Dir $verDir

$zipName = "bb_scrape_v${version}_${Arch}.zip"
$zipPath = Join-Path $verDir $zipName

if (Test-Path -LiteralPath $zipPath) { Remove-Item -LiteralPath $zipPath -Force }

$sevenZip = if ($UsePsZip) { $null } else { Get-SevenZipPath }

if ($sevenZip) {
  Write-Host "Using 7-Zip at $sevenZip"
  # Add files individually from their directories to put them at zip root
  foreach ($f in $files) {
    $fileName = Split-Path -Leaf $f
    & $sevenZip a -tzip -mx=9 -bd -- "$zipPath" "$f" | Out-Null
    & $sevenZip rn -bd -- "$zipPath" "$f" "$fileName" | Out-Null
  }
  Write-Host "Archive created with top-level files"
} else {
  Write-Host "7-Zip not found; using Compress-Archive"
  # For Compress-Archive, we need to copy files to temp dir to flatten structure
  $tempDir = Join-Path $env:TEMP "bb_scrape_zip_temp_$(Get-Random)"
  try {
    New-Item -ItemType Directory -Path $tempDir -Force | Out-Null
    foreach ($f in $files) {
      $fileName = Split-Path -Leaf $f
      Copy-Item -LiteralPath $f -Destination (Join-Path $tempDir $fileName)
    }
    Compress-Archive -Path (Join-Path $tempDir "*") -DestinationPath $zipPath -CompressionLevel Optimal -Force
  } finally {
    if (Test-Path $tempDir) { Remove-Item -Recurse -Force $tempDir }
  }
}

if (!(Test-Path -LiteralPath $zipPath)) {
  throw "Expected zip not found at $zipPath (archive step failed)"
}

$hash = Get-FileHash -LiteralPath $zipPath -Algorithm SHA256
$hashFile = "$zipPath.sha256"
"$($hash.Hash)  $zipName" | Set-Content -NoNewline -Encoding ASCII -Path $hashFile

Write-Host "Created: $zipPath"
Write-Host "SHA256:  $hashFile"

"ZIP=$zipPath"
"SHA256=$hashFile"

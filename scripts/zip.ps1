param(
  [string]$OutDir = "artifacts",
  [string]$CargoTomlPath = "Cargo.toml",
  [string]$Arch = "windows_x86_64",
  [string[]]$ExtraFiles = @(),
  [switch]$BuildRelease,
  [switch]$UsePsZip
)

$ErrorActionPreference = 'Stop'

function Get-VersionFromCargoToml([string]$path) {
  if (!(Test-Path $path)) { throw "Cargo.toml not found at '$path'" }
  $lines = Get-Content -Raw -LiteralPath $path -Encoding UTF8
  # Extract version under [package]
  $pkgSec = ($lines -split "\r?\n")
  $inPkg = $false
  foreach ($ln in $pkgSec) {
    if ($ln -match '^\s*\[package\]') { $inPkg = $true; continue }
    if ($inPkg -and $ln -match '^\s*\[') { break } # next section
    if ($inPkg -and $ln -match '^\s*version\s*=\s*"([^"]+)"') { return $Matches[1] }
  }
  throw "Could not find version in [package] section of $path"
}

function Ensure-Dir([string]$dir) {
  if (!(Test-Path $dir)) { [void](New-Item -ItemType Directory -Force -Path $dir) }
}

function Get-SevenZipPath() {
  try {
    $cmd = Get-Command 7z -ErrorAction Stop
    return $cmd.Path
  } catch {
    $p1 = Join-Path $Env:ProgramFiles '7-Zip\7z.exe'
    $p2 = Join-Path ${Env:ProgramFiles(x86)} '7-Zip\7z.exe'
    foreach ($p in @($p1,$p2)) { if (Test-Path $p) { return $p } }
    return $null
  }
}

# 1) Optional build
if ($BuildRelease) {
  Write-Host "Building release binaries..."
  & cargo build --release | Write-Host
  & cargo build --release --bin cli --features cli | Write-Host
}

# 2) Files to include (make this easy to tweak)
$baseFiles = @(
  'target\release\bb_scrape.exe',
  'target\release\cli.exe',
  'README.md',
  'LICENSE'
)
$files = @()
$files += $baseFiles
if ($ExtraFiles) { $files += $ExtraFiles }

# Validate inputs exist
foreach ($f in $files) {
  if (!(Test-Path $f)) { throw "File not found: $f" }
}

# 3) Version + output paths
$version = Get-VersionFromCargoToml -path $CargoTomlPath
Ensure-Dir $OutDir
$zipName = "bb_scrape_v$version`_$Arch.zip"
$zipPath = Join-Path $OutDir $zipName

# 4) Zip with 7z if present; fallback to Compress-Archive
$sevenZip = if ($UsePsZip) { $null } else { Get-SevenZipPath }
if ($sevenZip) {
  Write-Host "Using 7-Zip at $sevenZip"
  # -mx=9 max compression, -tzip zip format, -bd no progress indicator
  & $sevenZip a -tzip -mx=9 -bd -- "%zipPath%" @files
} else {
  Write-Host "7-Zip not found; using Compress-Archive"
  if (Test-Path $zipPath) { Remove-Item -Force $zipPath }
  Compress-Archive -Path $files -DestinationPath $zipPath -CompressionLevel Optimal -Force
}

# 5) SHA256
$hash = Get-FileHash -Path $zipPath -Algorithm SHA256
$hashFile = "$zipPath.sha256"
"$($hash.Hash)  $zipName" | Set-Content -NoNewline -Encoding ASCII -Path $hashFile

Write-Host "Created: $zipPath"
Write-Host "SHA256:  $hashFile"

# Return paths for CI usage
"ZIP=$zipPath"
"SHA256=$hashFile"


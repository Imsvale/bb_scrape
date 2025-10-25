# scripts/zip.ps1
param(
  [string]$OutDir = "artifacts",
  [string]$CargoTomlPath = "Cargo.toml",
  [string]$Arch = "windows_x86_64",
  [string[]]$ExtraFiles = @(),
  [switch]$BuildRelease,
  [switch]$UsePsZip
)

$ErrorActionPreference = "Stop"

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

if ($BuildRelease) {
  Write-Host "Building release binaries..."
  cargo build --release | Out-Host
  cargo build --release --bin cli --features cli | Out-Host
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
  if (!(Test-Path -LiteralPath $f)) { throw "File not found: $f" }
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
  & $sevenZip a -tzip -mx=9 -bd -- "$zipPath" @files | Out-Host
} else {
  Write-Host "7-Zip not found; using Compress-Archive"
  Compress-Archive -Path $files -DestinationPath $zipPath -CompressionLevel Optimal -Force
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

<#
.SYNOPSIS
  Build an MSIX package for the Microsoft Store.

.DESCRIPTION
  Rebuilds Mushak, stages the executable and package assets, inserts the Store
  package version into AppxManifest.xml, and runs MakeAppx validation.

  The application version comes from Cargo.toml. Store package versions use
  four numeric parts. Microsoft reserves the fourth part and rejects a zero
  first part, so Mushak 0.0.3 maps to package version 1.0.3.0 by default.

  The Store signs the package after certification. Use -Sign only for a local
  test installation with a certificate that this computer trusts.

.EXAMPLE
  pwsh packaging/msix/build-msix.ps1

.EXAMPLE
  pwsh packaging/msix/build-msix.ps1 -SkipBuild -Sign -CertSubject "CN=<publisher-id>"

.EXAMPLE
  pwsh packaging/msix/build-msix.ps1 -Version 0.0.4 -PackageVersion 1.0.4.0
#>
[CmdletBinding()]
param(
    [string]$Version,
    [string]$PackageVersion,
    [switch]$SkipBuild,
    [switch]$Sign,
    [string]$CertSubject
)

$ErrorActionPreference = 'Stop'
$msixDir = $PSScriptRoot
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path

# Read the application version from Cargo.toml unless the caller overrides it.
if (-not $Version) {
    $line = Select-String -Path (Join-Path $root 'Cargo.toml') -Pattern '^\s*version\s*=\s*"([^"]+)"' | Select-Object -First 1
    if (-not $line) { throw 'Could not read version from Cargo.toml' }
    $Version = $line.Matches[0].Groups[1].Value
}
if ($Version -notmatch '^(\d+)\.(\d+)\.(\d+)$') {
    throw "Application version must have three numeric parts: $Version"
}
$appParts = @(
    [uint64]$Matches[1],
    [uint64]$Matches[2],
    [uint64]$Matches[3]
)

# Adding one to the Cargo major keeps ordering monotonic and ensures the first
# Store package field is never zero while Mushak uses a pre-1.0 version.
if (-not $PackageVersion) {
    if ($appParts[0] -ge 65535) {
        throw "Application major version is too large for Store mapping: $Version"
    }
    $PackageVersion = "{0}.{1}.{2}.0" -f ($appParts[0] + 1), $appParts[1], $appParts[2]
}
if ($PackageVersion -notmatch '^(\d+)\.(\d+)\.(\d+)\.(\d+)$') {
    throw "Store package version must have four numeric parts: $PackageVersion"
}
$packageParts = @(
    [uint64]$Matches[1],
    [uint64]$Matches[2],
    [uint64]$Matches[3],
    [uint64]$Matches[4]
)
if ($packageParts[0] -eq 0) {
    throw "Store package version cannot start with zero: $PackageVersion"
}
if (($packageParts | Where-Object { $_ -gt 65535 }).Count -gt 0) {
    throw "Each Store package version part must be at most 65535: $PackageVersion"
}
if ($packageParts[3] -ne 0) {
    throw "Store package version must reserve the fourth part as zero: $PackageVersion"
}

Write-Host "Application version: $Version"
Write-Host "Store package version: $PackageVersion"

# Build the exact executable that will enter the package.
$exe = Join-Path $root 'target\release\mushak.exe'
if (-not $SkipBuild) {
    Write-Host 'Building release binary with Cargo.lock...'
    Push-Location $root
    try {
        cargo build --release --locked
        if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }
    }
    finally {
        Pop-Location
    }
}
if (-not (Test-Path $exe)) {
    throw "Release executable not found: $exe"
}
$binaryVersion = (Get-Item $exe).VersionInfo.ProductVersion
if ($binaryVersion -and $binaryVersion -ne $Version) {
    throw "Release executable version is $binaryVersion, expected $Version"
}

$makeappx = Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\bin\*\x64\makeappx.exe' -ErrorAction SilentlyContinue |
    Sort-Object FullName -Descending |
    Select-Object -First 1
if (-not $makeappx) { throw 'makeappx.exe not found. Install the Windows 10/11 SDK.' }

$stage = Join-Path $root 'target\msix\stage'
if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
New-Item -ItemType Directory -Force $stage | Out-Null
New-Item -ItemType Directory -Force (Join-Path $stage 'Assets') | Out-Null

$manifest = Get-Content (Join-Path $msixDir 'AppxManifest.xml') -Raw
# Keep the check case-sensitive so "replacement" in the description is safe.
if ($manifest -cmatch 'REPLACEME|PUT-[A-Z-]+-HERE') {
    throw 'AppxManifest.xml still contains identity placeholders'
}
$manifest = $manifest -replace 'Version="0\.0\.0\.0"', ("Version=""{0}""" -f $PackageVersion)
[xml]$manifestDocument = $manifest
$identity = $manifestDocument.Package.Identity
$identityName = $identity.GetAttribute('Name')
$identityPublisher = $identity.GetAttribute('Publisher')
$identityVersion = $identity.GetAttribute('Version')
$identityArchitecture = $identity.GetAttribute('ProcessorArchitecture')
if (-not $identityName -or -not $identityPublisher) {
    throw 'AppxManifest.xml is missing the Partner Center identity'
}
if ($identityVersion -ne $PackageVersion) {
    throw "Manifest version substitution failed: $identityVersion"
}
if ($identityArchitecture -ne 'x64') {
    throw "This build produces x64 code but the manifest declares $identityArchitecture"
}

Set-Content -Path (Join-Path $stage 'AppxManifest.xml') -Value $manifest -Encoding UTF8
Copy-Item $exe (Join-Path $stage 'mushak.exe')
Copy-Item (Join-Path $msixDir 'Assets\*') (Join-Path $stage 'Assets') -Recurse

$outDir = Join-Path $root 'target\msix'
$out = Join-Path $outDir ("mushak-{0}-x64.msix" -f $Version)
& $makeappx.FullName pack /d $stage /p $out /o
if ($LASTEXITCODE -ne 0) { throw 'makeappx failed' }

# Read the finished archive back through MakeAppx. This catches a corrupt
# package and lets us verify the manifest that will actually be uploaded.
$inspection = Join-Path $outDir 'inspection'
if (Test-Path $inspection) { Remove-Item -Recurse -Force $inspection }
& $makeappx.FullName unpack /p $out /d $inspection /o
if ($LASTEXITCODE -ne 0) { throw 'makeappx package inspection failed' }
[xml]$packedManifest = Get-Content (Join-Path $inspection 'AppxManifest.xml') -Raw
$packedIdentity = $packedManifest.Package.Identity
if ($packedIdentity.GetAttribute('Name') -ne $identityName -or
    $packedIdentity.GetAttribute('Publisher') -ne $identityPublisher -or
    $packedIdentity.GetAttribute('Version') -ne $PackageVersion -or
    $packedIdentity.GetAttribute('ProcessorArchitecture') -ne 'x64') {
    throw 'The packed manifest does not match the requested Store identity'
}
Write-Host "Packed: $out"

if ($Sign) {
    if (-not $CertSubject) {
        throw "-Sign requires -CertSubject 'CN=<publisher-id>' matching the manifest Publisher."
    }
    $signtool = Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\bin\*\x64\signtool.exe' -ErrorAction SilentlyContinue |
        Sort-Object FullName -Descending |
        Select-Object -First 1
    if (-not $signtool) { throw 'signtool.exe not found. Install the Windows 10/11 SDK.' }
    & $signtool.FullName sign /fd SHA256 /a /n ($CertSubject -replace '^CN=', '') $out
    if ($LASTEXITCODE -ne 0) { throw 'signtool failed' }
    Write-Host "Signed for local testing: $out"
}

$hash = (Get-FileHash $out -Algorithm SHA256).Hash
Write-Host "SHA256: $hash"

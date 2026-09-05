<#
.SYNOPSIS
  Build the Program Files MSI containing Mushak and its wheel-only UIAccess helper.

.DESCRIPTION
  A functional UIAccess build must be Authenticode-signed. Pass the SHA-1
  thumbprint of a code-signing certificate that Windows trusts. The script
  signs both executables before WiX embeds them, then signs the MSI.

  -AllowUnsigned exists only for structural CI/development validation. Windows
  will refuse UIAccess to that helper and Mushak will use native-wheel fallback.

.EXAMPLE
  pwsh packaging/wix/build-installer.ps1 -CertificateThumbprint ABCD1234...

.EXAMPLE
  pwsh packaging/wix/build-installer.ps1 -AllowUnsigned
#>
[CmdletBinding()]
param(
    [string]$Version,
    [string]$CertificateThumbprint,
    [string]$TimestampUrl = 'http://timestamp.digicert.com',
    [switch]$AllowUnsigned
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path

if (-not $Version) {
    $line = Select-String -Path (Join-Path $root 'Cargo.toml') -Pattern '^\s*version\s*=\s*"([^"]+)"' | Select-Object -First 1
    if (-not $line) { throw 'Could not read version from Cargo.toml.' }
    $Version = $line.Matches[0].Groups[1].Value
}
if (-not $CertificateThumbprint -and -not $AllowUnsigned) {
    throw 'A trusted code-signing -CertificateThumbprint is required. Use -AllowUnsigned only to validate package structure.'
}

Push-Location $root
try {
    cargo build --release --workspace
    if ($LASTEXITCODE -ne 0) { throw 'cargo release build failed' }

    $mainExe = Join-Path $root 'target\release\mushak.exe'
    $helperExe = Join-Path $root 'target\release\mushak-uiaccess-helper.exe'
    if (-not (Test-Path $mainExe) -or -not (Test-Path $helperExe)) {
        throw 'One or more release executables are missing.'
    }

    if ($CertificateThumbprint) {
        $signtool = Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\bin\*\x64\signtool.exe' -ErrorAction SilentlyContinue |
            Sort-Object FullName -Descending | Select-Object -First 1
        if (-not $signtool) { throw 'signtool.exe not found. Install the Windows 10/11 SDK.' }

        foreach ($binary in @($mainExe, $helperExe)) {
            $args = @('sign', '/fd', 'SHA256', '/sha1', $CertificateThumbprint)
            if ($TimestampUrl) { $args += @('/tr', $TimestampUrl, '/td', 'SHA256') }
            $args += $binary
            & $signtool.FullName @args
            if ($LASTEXITCODE -ne 0) { throw "Signing failed: $binary" }
        }
    } else {
        Write-Warning 'Building an unsigned structural MSI. The UIAccess helper will not run; native-wheel fallback remains active.'
    }

    $candle = Get-Command candle.exe -ErrorAction SilentlyContinue
    $light = Get-Command light.exe -ErrorAction SilentlyContinue
    if ($candle -and $light) {
        $candleExe = $candle.Source
        $lightExe = $light.Source
    } else {
        $wixDir = Join-Path $root 'target\tools\wix314'
        $candlePath = Join-Path $wixDir 'candle.exe'
        $lightPath = Join-Path $wixDir 'light.exe'
        if (-not (Test-Path $candlePath) -or -not (Test-Path $lightPath)) {
            $zip = Join-Path $root 'target\tools\wix314-binaries.zip'
            New-Item -ItemType Directory -Force (Split-Path $zip) | Out-Null
            Invoke-WebRequest -UseBasicParsing -OutFile $zip 'https://github.com/wixtoolset/wix3/releases/download/wix3141rtm/wix314-binaries.zip'
            New-Item -ItemType Directory -Force $wixDir | Out-Null
            Expand-Archive -Force $zip $wixDir
        }
        $candleExe = $candlePath
        $lightExe = $lightPath
    }

    $objDir = Join-Path $root 'target\wix'
    New-Item -ItemType Directory -Force $objDir | Out-Null
    & $candleExe "-dVersion=$Version" -ext WixUIExtension -arch x64 -out "$objDir\" (Join-Path $root 'packaging\wix\Mushak.wxs')
    if ($LASTEXITCODE -ne 0) { throw 'WiX candle failed' }

    $msi = Join-Path $root "mushak-$Version-x64.msi"
    & $lightExe -ext WixUIExtension -out $msi (Join-Path $objDir 'Mushak.wixobj')
    if ($LASTEXITCODE -ne 0) { throw 'WiX light failed' }

    if ($CertificateThumbprint) {
        $args = @('sign', '/fd', 'SHA256', '/sha1', $CertificateThumbprint)
        if ($TimestampUrl) { $args += @('/tr', $TimestampUrl, '/td', 'SHA256') }
        $args += $msi
        & $signtool.FullName @args
        if ($LASTEXITCODE -ne 0) { throw "Signing failed: $msi" }
    }

    $hash = (Get-FileHash $msi -Algorithm SHA256).Hash
    Write-Host "Built: $msi"
    Write-Host "SHA256: $hash"
    if (-not $CertificateThumbprint) {
        Write-Host 'UIAccess status: disabled (unsigned structural build)'
    }
}
finally {
    Pop-Location
}

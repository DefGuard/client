$ErrorActionPreference = 'Stop'

$tauriConfigPath = Join-Path $PSScriptRoot '..\..\tauri.conf.json'
$tauriConfig = Get-Content $tauriConfigPath -Raw | ConvertFrom-Json
$windowsConfig = $tauriConfig.bundle.windows

$thumbprint = $windowsConfig.certificateThumbprint

$timestampUrl = if ($env:DEFGUARD_WINDOWS_TIMESTAMP_URL) {
    $env:DEFGUARD_WINDOWS_TIMESTAMP_URL
} else {
    $windowsConfig.timestampUrl
}

$digestAlgorithm = if ($windowsConfig.digestAlgorithm) {
    $windowsConfig.digestAlgorithm.ToUpperInvariant()
} else {
    'SHA256'
}

if (-not $thumbprint) {
    throw 'Windows certificate thumbprint is not configured.'
}

$signtool = Get-Command signtool.exe -ErrorAction SilentlyContinue
if (-not $signtool) {
    throw 'signtool.exe was not found.'
}

$binaries = @(
    'target\release\defguard-cli.exe',
    'target\release\defguard-service.exe'
)

foreach ($binary in $binaries) {
    if (-not (Test-Path $binary)) {
        throw "Binary not found: $binary"
    }

    Write-Host "Signing $binary"
    & $signtool.Source sign /sha1 $thumbprint /fd $digestAlgorithm /tr $timestampUrl /td $digestAlgorithm $binary
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to sign $binary"
    }
}

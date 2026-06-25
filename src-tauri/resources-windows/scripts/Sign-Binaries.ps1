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

# Resolve signtool to a plain path string. Get-Command returns a
# CommandInfo (use .Source); the Windows Kits fallback returns a
# FileInfo (use .FullName).
$signtoolPath = (Get-Command signtool.exe -ErrorAction SilentlyContinue).Source
if (-not $signtoolPath) {
    $signtoolPath = Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\bin' -Recurse -Filter signtool.exe |
        Where-Object { $_.FullName -match '\\x64\\signtool\.exe$' } |
        Sort-Object FullName -Descending |
        Select-Object -First 1 -ExpandProperty FullName
}

if (-not $signtoolPath) {
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
    & $signtoolPath sign /sha1 $thumbprint /fd $digestAlgorithm /tr $timestampUrl /td $digestAlgorithm $binary
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to sign $binary"
    }
}

[CmdletBinding()]
param([switch]$DebugBuild, [switch]$SkipChecks, [string]$ServiceConfig, [switch]$RequireServiceConfig)
$ErrorActionPreference = 'Stop'
foreach ($command in @('node','npm.cmd','cargo','python')) {
    if (-not (Get-Command $command -ErrorAction SilentlyContinue)) { throw "Missing build prerequisite: $command. See README.md." }
}
$previousConfig = $env:OPENRIDESYNC_SERVICE_CONFIG
Push-Location $PSScriptRoot
try {
    if ($ServiceConfig) { $env:OPENRIDESYNC_SERVICE_CONFIG = [IO.File]::ReadAllText((Resolve-Path -LiteralPath $ServiceConfig)) }
    if ($RequireServiceConfig) {
        & python scripts/check-service-config.py --required --verify-provenance
    } else {
        & python scripts/check-service-config.py
    }
    if ($LASTEXITCODE -ne 0) { throw 'Service configuration validation failed; values are never logged.' }
    & python scripts/check-source.py
    if ($LASTEXITCODE -ne 0) { throw 'Source and version checks failed.' }
    & npm.cmd ci
    if ($LASTEXITCODE -ne 0) { throw 'Dependency installation failed.' }
    if (-not $SkipChecks) {
        & python -m unittest discover -s tests -p '*_test.py'
        if ($LASTEXITCODE -ne 0) { throw 'Build configuration checks failed.' }
        & npm.cmd test
        if ($LASTEXITCODE -ne 0) { throw 'Frontend checks failed.' }
        & cargo fmt --all --check --manifest-path src-tauri/Cargo.toml
        if ($LASTEXITCODE -ne 0) { throw 'Rust formatting failed.' }
        & cargo test --locked --manifest-path src-tauri/Cargo.toml
        if ($LASTEXITCODE -ne 0) { throw 'Native checks failed.' }
        & cargo clippy --locked --all-targets --manifest-path src-tauri/Cargo.toml -- -D warnings
        if ($LASTEXITCODE -ne 0) { throw 'Rust lint failed.' }
    }
    $buildArgs = @('run','tauri','--','build','--no-bundle')
    if ($DebugBuild) { $buildArgs += '--debug' }
    $buildArgs += @('--','--locked')
    & npm.cmd @buildArgs
    if ($LASTEXITCODE -ne 0) { throw 'Tauri build failed.' }
    $profile = if ($DebugBuild) { 'debug' } else { 'release' }
    $destination = Join-Path $PSScriptRoot $(if ($DebugBuild) { 'release/debug' } else { 'release' })
    [IO.Directory]::CreateDirectory($destination) | Out-Null
    $executable = Join-Path $destination 'OpenRideSync Viewer.exe'
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot "src-tauri/target/$profile/open-roadsync-viewer.exe") -Destination $executable
    foreach ($file in @('LICENSE','README.md','THIRD-PARTY-NOTICES.md','service-config.example.json','service-provenance.json')) {
        Copy-Item -LiteralPath (Join-Path $PSScriptRoot $file) -Destination $destination
    }
    & python scripts/collect-licenses.py (Join-Path $destination 'THIRD-PARTY-LICENSES.txt')
    if ($LASTEXITCODE -ne 0) { throw 'Third-party license collection failed.' }
    $version = (Get-Content -Raw package.json | ConvertFrom-Json).version
    $archive = Join-Path $destination "OpenRideSync-Viewer-$version-windows-x64.zip"
    $packageFiles = @('OpenRideSync Viewer.exe','LICENSE','README.md','THIRD-PARTY-NOTICES.md','THIRD-PARTY-LICENSES.txt','service-config.example.json','service-provenance.json') | ForEach-Object { Join-Path $destination $_ }
    Compress-Archive -LiteralPath $packageFiles -DestinationPath $archive -Force
    foreach ($artifact in @($executable,$archive)) {
        $hash = (Get-FileHash -LiteralPath $artifact -Algorithm SHA256).Hash.ToLowerInvariant()
        [IO.File]::WriteAllText(($artifact + '.sha256'), "$hash  $([IO.Path]::GetFileName($artifact))" + [char]10, [Text.Encoding]::ASCII)
    }
    Write-Host "Build completed: $archive"
} finally { $env:OPENRIDESYNC_SERVICE_CONFIG = $previousConfig; Pop-Location }

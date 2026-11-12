$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$cargoBin = Join-Path $env:USERPROFILE '.cargo/bin'
$localCompiler = Join-Path $repoRoot '.tools/mingw64/bin'
$env:PATH = "$cargoBin;$localCompiler;$env:PATH"
Push-Location $repoRoot
try {
    cargo build --workspace --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'Rust build failed' }
    Write-Host "Desktop: $repoRoot/target/release/openmbs-desktop.exe"
    Write-Host "CLI: $repoRoot/target/release/openmbs.exe"
} finally { Pop-Location }

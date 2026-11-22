[CmdletBinding()]
param([ValidateSet('dev','build','test')][string]$Task='dev')
$ErrorActionPreference='Stop'
$projectRoot=Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $projectRoot
$oldPath=$env:PATH
$oldCargo=$env:CARGO_HOME
$oldRustup=$env:RUSTUP_HOME
try {
    $portable=Join-Path $projectRoot '.work\rustup\toolchains\stable-x86_64-pc-windows-msvc\bin'
    if (Test-Path -LiteralPath (Join-Path $portable 'cargo.exe')) {
        $env:RUSTUP_HOME=Join-Path $projectRoot '.work\rustup'
        $env:CARGO_HOME=Join-Path $projectRoot '.work\cargo'
        $env:PATH="$portable;$env:PATH"
    }
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {throw 'Install the Rust MSVC toolchain. See docs/DEVELOPMENT.md.'}
    if (-not (Get-Command pnpm -ErrorAction SilentlyContinue)) {throw 'Install pnpm. See docs/DEVELOPMENT.md.'}
    switch ($Task) {
        dev { pnpm desktop }
        build { pnpm bundle }
        test {
            pnpm build
            if ($LASTEXITCODE -ne 0) {throw 'Frontend build failed.'}
            cargo test --workspace --locked --release
        }
    }
    if ($LASTEXITCODE -ne 0) {throw "$Task failed with exit code $LASTEXITCODE."}
} finally {
    $env:PATH=$oldPath
    $env:CARGO_HOME=$oldCargo
    $env:RUSTUP_HOME=$oldRustup
    Pop-Location
}

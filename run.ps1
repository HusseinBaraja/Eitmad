[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repositoryRoot = $PSScriptRoot
$shellProject = Join-Path $repositoryRoot "shells\windows\Eitmad.WindowsShell.csproj"

Push-Location $repositoryRoot
try {
    & cargo build -p eitmad-engine-cli
    if ($LASTEXITCODE -ne 0) {
        throw "The Rust engine build failed with exit code $LASTEXITCODE."
    }

    $cargoMetadata = & cargo metadata --format-version 1 --no-deps | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) {
        throw "Cargo metadata failed with exit code $LASTEXITCODE."
    }

    $enginePath = Join-Path $cargoMetadata.target_directory "debug\eitmad-engine-cli.exe"
    & $enginePath seed-development-accounts
    if ($LASTEXITCODE -ne 0) {
        throw "Development account provisioning failed with exit code $LASTEXITCODE."
    }

    Write-Host "Development accounts:"
    Write-Host "  Manager      test.manager / Eitmad-Manager-2026!"
    Write-Host "  Receptionist test.receptionist / Eitmad-Reception-2026!"

    & dotnet run --project $shellProject -- --engine $enginePath
    if ($LASTEXITCODE -ne 0) {
        throw "The Windows app stopped with exit code $LASTEXITCODE."
    }
}
finally {
    Pop-Location
}

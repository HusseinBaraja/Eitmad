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
    & dotnet run --project $shellProject -- --engine $enginePath
    if ($LASTEXITCODE -ne 0) {
        throw "The Windows app stopped with exit code $LASTEXITCODE."
    }
}
finally {
    Pop-Location
}

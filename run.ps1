[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repositoryRoot = $PSScriptRoot
$enginePath = Join-Path $repositoryRoot "target\debug\eitmad-engine-cli.exe"
$shellProject = Join-Path $repositoryRoot "shells\windows\Eitmad.WindowsShell.csproj"

Push-Location $repositoryRoot
try {
    & cargo build -p eitmad-engine-cli
    if ($LASTEXITCODE -ne 0) {
        throw "The Rust engine build failed with exit code $LASTEXITCODE."
    }

    & dotnet run --project $shellProject -- --engine $enginePath
    if ($LASTEXITCODE -ne 0) {
        throw "The Windows app stopped with exit code $LASTEXITCODE."
    }
}
finally {
    Pop-Location
}

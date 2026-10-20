[CmdletBinding()]
param(
    [string]$RuntimeDirectory,
    [switch]$SkipBuild
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repositoryRoot = $PSScriptRoot
$shellProject = Join-Path $repositoryRoot "shells\windows\Eitmad.WindowsShell.csproj"
$previousPath = $env:PATH
$previousTarget = $env:CARGO_TARGET_DIR
$previousIncremental = $env:CARGO_INCREMENTAL

if ($RuntimeDirectory) {
    $RuntimeDirectory = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($RuntimeDirectory)
}

Push-Location $repositoryRoot
try {
    $cargoPath = & rustup which --toolchain 1.85.1 cargo
    if ($LASTEXITCODE -ne 0) {
        throw 'Install Rust 1.85.1 with rustup.'
    }
    $env:PATH = (Split-Path $cargoPath -Parent) + [IO.Path]::PathSeparator + $previousPath
    $env:CARGO_TARGET_DIR = Join-Path $repositoryRoot 'target/toolchain-1.85.1'
    $env:CARGO_INCREMENTAL = '0'
    if (-not $SkipBuild) {
        & rustup run 1.85.1 cargo build --locked -p eitmad-engine-cli
        if ($LASTEXITCODE -ne 0) {
            throw "The Rust engine build failed with exit code $LASTEXITCODE."
        }
        & dotnet build $shellProject --configuration Release --nologo
        if ($LASTEXITCODE -ne 0) {
            throw "The Windows shell build failed with exit code $LASTEXITCODE."
        }
    }

    $enginePath = Join-Path $env:CARGO_TARGET_DIR "debug\eitmad-engine-cli.exe"
    $shellPath = Join-Path $repositoryRoot 'shells/windows/bin/Release/net8.0-windows/Eitmad.WindowsShell.exe'
    foreach ($file in $enginePath, $shellPath) {
        if (-not (Test-Path -LiteralPath $file -PathType Leaf)) {
            throw 'A build output is missing. Run without SkipBuild.'
        }
    }
    $shellArguments = @('--engine', $enginePath)
    if ($RuntimeDirectory) {
        $shellArguments += @('--runtime-directory', $RuntimeDirectory)
    }

    & $shellPath @shellArguments
    if ($LASTEXITCODE -ne 0) {
        throw "The Windows app stopped with exit code $LASTEXITCODE."
    }
}
finally {
    $env:PATH = $previousPath
    $env:CARGO_TARGET_DIR = $previousTarget
    $env:CARGO_INCREMENTAL = $previousIncremental
    Pop-Location
}

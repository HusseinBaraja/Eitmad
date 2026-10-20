#Requires -Version 5.1
param([Parameter(Mandatory)][string]$Action, [Parameter(Mandatory)][string]$StateDirectory, [string]$Backend)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
try {
    Import-Module (Join-Path $PSScriptRoot 'local-backup.psm1') -Force -DisableNameChecking
    $inputText = [Console]::In.ReadToEnd()
    if ($inputText.Length -gt 65536) { throw 'CaptureFailed' }
    $request = $inputText | ConvertFrom-Json
    $ids = @(Invoke-LocalBackupAction $Action $StateDirectory $Backend $request)
    [Console]::Out.WriteLine((@{outcome='succeeded'; generations=$ids} | ConvertTo-Json -Compress))
} catch {
    $outcome = if ($_.Exception.Message -in 'InsufficientSpace', 'DestinationUnavailable', 'RetentionFailed', 'ScheduleFailed') {
        $_.Exception.Message.Substring(0,1).ToLowerInvariant() + $_.Exception.Message.Substring(1)
    } else { 'captureFailed' }
    # Only typed observations cross back to Rust. Never forward exception/native text.
    [Console]::Out.WriteLine((@{outcome=$outcome; generations=@()} | ConvertTo-Json -Compress))
}

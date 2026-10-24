#Requires -Version 5.1
param([Parameter(Mandatory)][string]$Action, [Parameter(Mandatory)][string]$StateDirectory, [Parameter(Mandatory)][string]$Backend)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$env:PSModulePath = Join-Path $PSHOME 'Modules'
[Console]::InputEncoding = New-Object Text.UTF8Encoding($false, $true)
[Console]::OutputEncoding = New-Object Text.UTF8Encoding($false)
try {
    Import-Module (Join-Path $PSScriptRoot 'tls-renewal.psm1') -Force -DisableNameChecking
    $text = [Console]::In.ReadToEnd()
    if ($text.Length -gt 16384) { throw 'failed' }
    $request = $text | ConvertFrom-Json
    $text = $null
    $result = Invoke-TlsMechanic $Action $StateDirectory $request $Backend
    [Console]::Out.WriteLine((@{outcome='succeeded'; result=$result} | ConvertTo-Json -Depth 12 -Compress))
} catch {
    $outcome = if ($_.Exception.Message -eq 'credentialUnavailable') { 'credentialUnavailable' } else { 'failed' }
    [Console]::Out.WriteLine((@{outcome=$outcome} | ConvertTo-Json -Compress))
} finally { $request = $null; $text = $null }

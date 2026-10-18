#Requires -Version 7.4
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$TestRoot,
    [Parameter(Mandatory)][string]$ServerExecutable
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$launcher = Join-Path $PSScriptRoot 'lan-server.ps1'
$state = Join-Path $TestRoot ('launcher-' + [Guid]::NewGuid().ToString('N'))
# This script consumes only synthetic certificate-test state. It never changes firewall policy.
if (-not (Test-Path -LiteralPath (Join-Path $TestRoot 'server/previous-tls.json'))) {
    throw 'Run the certificate lifecycle tests in a new EITMAD_LAN_TEST_ROOT first.'
}
Add-Type -Path (Join-Path $PSScriptRoot 'lan-certificate/bin/Release/net8.0-windows/Eitmad.LanCertificate.dll')
[Eitmad.LanCertificate.Certificates]::Initialize($state, '192.168.50.10', 'synthetic launcher CA password')
$env:EITMAD_SERVER_DATABASE_URL = 'postgresql://synthetic:synthetic@127.0.0.1:55439/synthetic'
$env:EITMAD_SERVER_TOKEN_KEY = [Convert]::ToBase64String([byte[]](1..32)).TrimEnd('=').Replace('+', '-').Replace('/', '_')
$env:EITMAD_SERVER_UPDATE_MANIFEST_DIRECTORY = $TestRoot
$env:EITMAD_SERVER_UPDATE_OPERATOR_TENANT_ID = [Guid]::NewGuid().ToString()
$env:EITMAD_SERVER_UPDATE_KEY_ID = 'synthetic-lan-test'
$env:EITMAD_SERVER_UPDATE_PUBLIC_KEY = '11qYAYKxCrfVS/7TyWQHOg7hcvPapiMlrwIaaPcHURo='
$env:EITMAD_SERVER_ALLOW_INSECURE_LOOPBACK = 'true'
$originalToken = $env:EITMAD_SERVER_TOKEN_KEY

function Expect-Failure([scriptblock]$Operation, [string]$Expected) {
    $caught = $false
    try { & $Operation } catch {
        $caught = $true
        if ($_.Exception.Message -notlike "*$Expected*") { throw }
    }
    if (-not $caught) { throw "Expected rejection: $Expected" }
}

& $launcher Save -StateDirectory $state -ServerExecutable $ServerExecutable -ListenAddress '192.168.50.10'
if ($env:EITMAD_SERVER_TOKEN_KEY -cne $originalToken -or $env:EITMAD_SERVER_ALLOW_INSECURE_LOOPBACK -cne 'true') {
    throw 'Launcher did not restore the caller environment.'
}
$saved = [IO.File]::ReadAllBytes((Join-Path $state 'server.json'))
Expect-Failure { & $launcher Save -StateDirectory $state -ServerExecutable $ServerExecutable -ListenAddress '192.168.50.10' } 'already exist'
Get-ChildItem Env:EITMAD_SERVER_* | Remove-Item
& $launcher Check -StateDirectory $state
if (Test-Path Env:EITMAD_SERVER_TOKEN_KEY) { throw 'Retained secret leaked into caller environment.' }

$firewallCapture = [pscustomobject]@{ Rule = $null }
function Get-NetFirewallRule { param($Name, $ErrorAction) return $null }
function New-NetFirewallRule {
    param($Name, $DisplayName, $Direction, $Action, $Profile, $Protocol, $LocalAddress,
        $LocalPort, $RemoteAddress, $Program, $EdgeTraversalPolicy, [switch]$WhatIf)
    $firewallCapture.Rule = $PSBoundParameters
}
& $launcher Firewall -StateDirectory $state -ClientAddress '192.168.50.21', '192.168.50.22' -WhatIf
if ($firewallCapture.Rule.Profile -ne 'Private' -or $firewallCapture.Rule.LocalAddress -ne '192.168.50.10' -or
    $firewallCapture.Rule.LocalPort -ne 8443 -or $firewallCapture.Rule.Protocol -ne 'TCP' -or
    $firewallCapture.Rule.Direction -ne 'Inbound' -or $firewallCapture.Rule.Action -ne 'Allow' -or
    $firewallCapture.Rule.EdgeTraversalPolicy -ne 'Block' -or -not $firewallCapture.Rule.WhatIf -or
    $firewallCapture.Rule.Program -ne (Resolve-Path -LiteralPath $ServerExecutable).Path -or
    ($firewallCapture.Rule.RemoteAddress -join ',') -ne '192.168.50.21,192.168.50.22') {
    throw 'Firewall scope differs from the accepted client-only HTTPS rule.'
}
foreach ($invalid in @('Any', '0.0.0.0/0', '192.168.0.0/16', '8.8.8.8')) {
    Expect-Failure { & $launcher Firewall -StateDirectory $state -ClientAddress $invalid -WhatIf } ''
}
function Get-NetTCPConnection { param($State, $LocalPort, $ErrorAction) [pscustomobject]@{ LocalAddress = '0.0.0.0' } }
Expect-Failure { & $launcher Start -StateDirectory $state } 'listen only on loopback'
if (-not [Linq.Enumerable]::SequenceEqual([byte[]]$saved, [IO.File]::ReadAllBytes((Join-Path $state 'server.json')))) {
    throw 'A rejected operation changed retained settings.'
}
Write-Host 'PASS: retained Rust configuration, environment restoration, firewall scope, and public-database rejection.'

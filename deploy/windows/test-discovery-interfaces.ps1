#Requires -Version 5.1
# Only synthetic CIM responses. No live network, profile, or firewall calls.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$failClass = $null
function Get-CimInstance {
    param($Namespace, $ClassName, $Filter)
    if ($ClassName -eq $failClass) { throw [InvalidOperationException]::new('Synthetic provider failure.') }
    switch ($ClassName) {
        MSFT_NetConnectionProfile {
            @(
                [pscustomobject]@{InterfaceIndex=1; NetworkCategory=1},
                [pscustomobject]@{InterfaceIndex=2; NetworkCategory=0},
                [pscustomobject]@{InterfaceIndex=3; NetworkCategory=2},
                [pscustomobject]@{InterfaceIndex=5; NetworkCategory=1},
                [pscustomobject]@{InterfaceIndex=5; NetworkCategory=0},
                [pscustomobject]@{InterfaceIndex=7; NetworkCategory=99}
            )
        }
        MSFT_NetIPAddress {
            foreach ($index in 1..8) {
                [pscustomobject]@{InterfaceIndex=$index; IPAddress="192.168.$index.10"; PrefixLength=24}
            }
        }
        MSFT_NetAdapter {
            foreach ($index in 1..7) {
                [pscustomobject]@{InterfaceIndex=$index; InterfaceOperationalStatus=$(if ($index -eq 6) { 2 } else { 1 })}
            }
        }
        default { throw 'Unexpected CIM provider.' }
    }
}
$adapter = Join-Path $PSScriptRoot '../../crates/server-discovery/src/interfaces.ps1'
$rows = (& $adapter) | ConvertFrom-Json
if ($rows.Count -ne 6 -or @($rows | Where-Object { $_.private }).Count -ne 1 -or
    -not $rows[0].private -or $rows[0].index -ne 1) {
    throw 'Only active interfaces with exclusively Private profiles may qualify.'
}
$failClass = 'MSFT_NetAdapter'
$failed = $false
$partial = @()
try { $partial = @(& $adapter) } catch [InvalidOperationException] { $failed = $true }
if (-not $failed -or $partial.Count) { throw 'Provider failure must not return partial interface policy.' }
Write-Host 'PASS: Private, Public, Domain, unknown, mixed-profile, disconnected, missing-adapter, and failed-provider policy.'

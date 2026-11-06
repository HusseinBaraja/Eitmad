#Requires -Version 5.1
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateSet('server', 'client')][string]$Role,
    [Parameter(Mandatory)][ValidateSet('CheckUser', 'User', 'Machine')][string]$Phase,
    [Parameter(Mandatory)][ValidatePattern('^[A-Fa-f0-9]{64}$')][string]$PublisherCertificateSha256
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$env:PSModulePath = Join-Path $PSHOME 'Modules'
$leases = New-Object 'Collections.Generic.List[IDisposable]'
try {
    # The signed Inno uninstaller pins the publisher. Verify and retain every
    # script before importing it, including when another UAC account runs this.
    for ($parent = $PSScriptRoot; $parent; $parent = Split-Path $parent -Parent) {
        if ((Get-Item -LiteralPath $parent -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) {
            throw 'Uninstall tools cannot use reparse points.'
        }
    }
    foreach ($name in 'uninstall.ps1', 'uninstallation.psm1', 'host.psm1', 'package.psm1', 'verify-offline.ps1') {
        $path = Join-Path $PSScriptRoot $name
        if ((Get-Item -LiteralPath $path -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) {
            throw 'Uninstall tools cannot use reparse points.'
        }
        $leases.Add([IO.File]::Open($path, 'Open', 'Read', 'Read'))
        $signature = Get-AuthenticodeSignature -LiteralPath $path
        if ($signature.Status -ne 'Valid') { throw 'Uninstall tool signature is not trusted.' }
        $hash = [Security.Cryptography.SHA256]::Create()
        try { $fingerprint = [BitConverter]::ToString($hash.ComputeHash($signature.SignerCertificate.RawData)).Replace('-', '') }
        finally { $hash.Dispose() }
        if ($fingerprint -ine $PublisherCertificateSha256) { throw 'Uninstall tool publisher differs.' }
    }
    Import-Module (Join-Path $PSScriptRoot 'uninstallation.psm1') -Force
    $bundle = $PSScriptRoot
    for ($level = 0; $level -lt 4; $level++) { $bundle = Split-Path $bundle -Parent }
    Invoke-EitmadUninstall -Role $Role -Phase $Phase -BundleDirectory $bundle -PublisherCertificateSha256 $PublisherCertificateSha256 -Publisher $signature.SignerCertificate.Subject
    exit 0
} catch {
    # Never print settings, credentials, raw database paths, or environment contents.
    [Console]::Error.WriteLine('Eitmad removal stopped. Close the app and verify owned services, paths, and signed setup files before retrying.')
    exit 1
} finally {
    foreach ($lease in $leases) { $lease.Dispose() }
}

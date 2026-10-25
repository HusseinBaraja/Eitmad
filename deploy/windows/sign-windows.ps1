#Requires -Version 7.4
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Path,
    [Parameter(Mandatory)][string]$Publisher,
    [Parameter(Mandatory)][string]$SigningHook
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'signing-certificate.psm1') -Force
$certificate = Get-EitmadSigningCertificate
Assert-EitmadSelfSignedCertificate $certificate $Publisher
# Sign locally with the retained Windows key. No timestamp service is required.
$files = if (Test-Path -LiteralPath $Path -PathType Container) {
    Get-ChildItem -LiteralPath $Path -Recurse -File | Where-Object Extension -In '.exe', '.dll', '.ps1', '.psm1', '.msix', '.cat'
} else { Get-Item -LiteralPath $Path }
foreach ($file in $files) {
    $signature = Get-AuthenticodeSignature -LiteralPath $file.FullName
    $owned = $file.Name -like 'Eitmad*' -or $file.Extension -in '.ps1', '.psm1', '.msix', '.cat'
    if ($signature.Status -ne 'Valid' -or ($owned -and $signature.SignerCertificate.Thumbprint -cne $certificate.Thumbprint)) {
        & $SigningHook -Path $file.FullName
        if (-not $?) { throw 'External signing hook failed.' }
        $signature = Get-AuthenticodeSignature -LiteralPath $file.FullName
    }
    if ($signature.Status -ne 'Valid') { throw 'Signing verification failed: require trusted Authenticode.' }
    if ($owned) {
        Assert-EitmadSelfSignedCertificate $signature.SignerCertificate $Publisher
        if ($signature.SignerCertificate.Thumbprint -cne $certificate.Thumbprint) {
            throw 'Signing verification failed: publisher certificate differs from the selected self-signed key.'
        }
    }
}

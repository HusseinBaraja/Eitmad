# Embedded into the native adapter and Rust backend. Never load this script from an unverified bundle.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$env:PSModulePath = Join-Path $PSHOME 'Modules'
Import-Module (Join-Path $PSHOME 'Modules/Microsoft.PowerShell.Security/Microsoft.PowerShell.Security.psd1') -Force
$inputData = [Console]::In.ReadToEnd() | ConvertFrom-Json
$catalog = Join-Path $inputData.root 'offline.cat'
$anchor = Microsoft.PowerShell.Security\Get-AuthenticodeSignature -LiteralPath $inputData.anchor
$signature = Microsoft.PowerShell.Security\Get-AuthenticodeSignature -LiteralPath $catalog
if ($anchor.Status -ne 'Valid' -or -not $anchor.TimeStamperCertificate -or
    $signature.Status -ne 'Valid' -or -not $signature.TimeStamperCertificate -or
    $signature.SignerCertificate.Subject -cne $anchor.SignerCertificate.Subject) {
    throw 'Require trusted, timestamped setup and catalog with the same publisher.'
}
if ($inputData.publisher -and $signature.SignerCertificate.Subject -cne $inputData.publisher) {
    throw 'Offline publisher differs from the approved release.'
}
$result = Microsoft.PowerShell.Security\Test-FileCatalog -Path $inputData.root -CatalogFilePath $catalog -Detailed
if ($result.Status -ne 'Valid' -or $result.HashAlgorithm -ne 'SHA256') {
    throw 'Offline payload contains changed, missing, or unexpected files.'
}

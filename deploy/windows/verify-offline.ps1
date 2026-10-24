# Embedded into the native adapter and Rust backend. Never load this script from an unverified bundle.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$env:PSModulePath = Join-Path $PSHOME 'Modules'
Import-Module (Join-Path $PSHOME 'Modules/Microsoft.PowerShell.Security/Microsoft.PowerShell.Security.psd1') -Force
$inputData = [Console]::In.ReadToEnd() | ConvertFrom-Json
$catalog = Join-Path $inputData.root 'offline.cat'
$anchor = Microsoft.PowerShell.Security\Get-AuthenticodeSignature -LiteralPath $inputData.anchor
$signature = Microsoft.PowerShell.Security\Get-AuthenticodeSignature -LiteralPath $catalog
if ($anchor.Status -ne 'Valid' -or $signature.Status -ne 'Valid' -or
    [Convert]::ToBase64String($signature.SignerCertificate.RawData) -cne
        [Convert]::ToBase64String($anchor.SignerCertificate.RawData)) {
    throw 'Require trusted setup and catalog signed with the same self-signed publisher certificate.'
}
$certificate = $anchor.SignerCertificate
$eku = @($certificate.Extensions | Where-Object { $_.Oid.Value -eq '2.5.29.37' })
$constraints = @($certificate.Extensions | Where-Object { $_.Oid.Value -eq '2.5.29.19' })
if ($certificate.Subject -cne $certificate.Issuer -or $certificate.NotBefore -gt (Get-Date) -or
    $certificate.NotAfter -le (Get-Date) -or $eku.Count -ne 1 -or $eku[0].EnhancedKeyUsages.Count -ne 1 -or
    $eku[0].EnhancedKeyUsages[0].Value -cne '1.3.6.1.5.5.7.3.3' -or
    $constraints.Count -ne 1 -or $constraints[0].CertificateAuthority) {
    throw 'Require a valid self-signed code-signing-only leaf certificate.'
}
if ($inputData.publisher -and $signature.SignerCertificate.Subject -cne $inputData.publisher) {
    throw 'Offline publisher differs from the approved release.'
}
$result = Microsoft.PowerShell.Security\Test-FileCatalog -Path $inputData.root -CatalogFilePath $catalog -Detailed
if ($result.Status -ne 'Valid' -or $result.HashAlgorithm -ne 'SHA256') {
    throw 'Offline payload contains changed, missing, or unexpected files.'
}

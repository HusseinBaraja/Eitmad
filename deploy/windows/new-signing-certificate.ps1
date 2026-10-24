#Requires -Version 7.4
[CmdletBinding()]
param([ValidateNotNullOrEmpty()][string]$Publisher = 'CN=Eitmad')
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'signing-certificate.psm1') -Force
$certificate = New-SelfSignedCertificate -Type Custom -Subject $Publisher -FriendlyName 'Eitmad publisher signing' `
    -CertStoreLocation 'Cert:\CurrentUser\My' -KeyAlgorithm RSA -KeyLength 3072 -HashAlgorithm SHA256 `
    -KeyExportPolicy NonExportable -KeyUsage DigitalSignature -NotAfter (Get-Date).AddYears(3) `
    -TextExtension @('2.5.29.37={text}1.3.6.1.5.5.7.3.3', '2.5.29.19={text}')
Assert-EitmadSelfSignedCertificate $certificate $Publisher
# Authenticode requires root trust even for a non-CA code-signing leaf.
# Add only the public certificate; keep its non-exportable private key in My.
$public = [Security.Cryptography.X509Certificates.X509Certificate2]::new($certificate.RawData)
$store = [Security.Cryptography.X509Certificates.X509Store]::new('Root', 'CurrentUser')
try { $store.Open('ReadWrite'); $store.Add($public) } finally { $store.Dispose(); $public.Dispose() }
[pscustomobject]@{
    publisher = $certificate.Subject
    certificateThumbprint = $certificate.Thumbprint
    certificateSha256 = $certificate.GetCertHashString([Security.Cryptography.HashAlgorithmName]::SHA256).ToLowerInvariant()
    expiresUtc = $certificate.NotAfter.ToUniversalTime().ToString('o')
} | ConvertTo-Json

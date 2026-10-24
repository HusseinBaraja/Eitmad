#Requires -Version 7.4
[CmdletBinding()]
param()
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repository = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
Import-Module (Join-Path $PSScriptRoot 'signing-certificate.psm1') -Force
Import-Module (Join-Path $PSScriptRoot 'package.psm1') -Force
$root = Join-Path ([IO.Path]::GetTempPath()) ('eitmad-self-signed-proof-' + [guid]::NewGuid().ToString('N'))
[void][IO.Directory]::CreateDirectory($root)
$subject = 'CN=Eitmad signing proof ' + [guid]::NewGuid().ToString('N')
$previousThumbprint = $env:EITMAD_SIGNING_CERTIFICATE_THUMBPRINT
$certificate = $null
function Reject([scriptblock]$Operation) {
    $rejected = $false
    try { & $Operation } catch { $rejected = $true }
    if (-not $rejected) { throw 'An invalid signing operation was accepted.' }
}
try {
    $result = & (Join-Path $PSScriptRoot 'new-signing-certificate.ps1') -Publisher $subject | ConvertFrom-Json
    $certificate = Get-Item -LiteralPath ('Cert:\CurrentUser\My\' + $result.certificateThumbprint)
    $env:EITMAD_SIGNING_CERTIFICATE_THUMBPRINT = $certificate.Thumbprint
    $publicPath = Join-Path $root 'publisher.cer'
    $hook = Join-Path $PSScriptRoot 'sign-with-self-signed-certificate.ps1'
    & $hook -ExportCertificate $publicPath
    $public = [Security.Cryptography.X509Certificates.X509Certificate2]::new([IO.File]::ReadAllBytes($publicPath))
    try {
        if ($public.HasPrivateKey -or $public.Thumbprint -cne $certificate.Thumbprint) { throw 'Public export differs.' }
        Assert-EitmadSelfSignedCertificate $public $subject
        Reject { Assert-EitmadSelfSignedCertificate $public 'CN=Wrong publisher' }
    } finally { $public.Dispose() }
    $executable = Join-Path $root 'EitmadSetup.exe'
    Copy-Item -LiteralPath (Join-Path $repository 'shells/windows-setup/bin/Release/net8.0-windows/EitmadSetup.exe') -Destination $executable
    $adapter = Join-Path $repository 'platform-adapters/windows/bin/Release/net8.0/Eitmad.Platform.Windows.dll'
    Add-Type -Path $adapter
    Reject { [Eitmad.Platform.Windows.Installation.AuthenticodeTrust]::Verify($executable) }
    & (Join-Path $PSScriptRoot 'sign-windows.ps1') -Path $executable -Publisher $subject -SigningHook $hook
    $signature = Get-AuthenticodeSignature -LiteralPath $executable
    if ($signature.Status -ne 'Valid' -or $signature.TimeStamperCertificate) { throw 'Expected trusted signing without a timestamp.' }
    [Eitmad.Platform.Windows.Installation.AuthenticodeTrust]::Verify($executable)
    $trust = [Eitmad.Platform.Windows.Installation.PublisherCertificateTrust]::new(
        [IO.File]::ReadAllBytes($publicPath), $result.certificateSha256)
    try { $trust.VerifyInstallerSigner($executable) } finally { $trust.Dispose() }
    # A different key with the same subject cannot authenticate this installer.
    $rsa = [Security.Cryptography.RSA]::Create(2048)
    try {
        $otherRequest = [Security.Cryptography.X509Certificates.CertificateRequest]::new(
            $subject, $rsa, [Security.Cryptography.HashAlgorithmName]::SHA256, [Security.Cryptography.RSASignaturePadding]::Pkcs1)
        $usages = [Security.Cryptography.OidCollection]::new()
        [void]$usages.Add([Security.Cryptography.Oid]::new('1.3.6.1.5.5.7.3.3'))
        $otherRequest.CertificateExtensions.Add([Security.Cryptography.X509Certificates.X509EnhancedKeyUsageExtension]::new($usages,$true))
        $otherRequest.CertificateExtensions.Add([Security.Cryptography.X509Certificates.X509BasicConstraintsExtension]::new($false,$false,0,$true))
        $other = $otherRequest.CreateSelfSigned([DateTimeOffset]::UtcNow.AddMinutes(-1),[DateTimeOffset]::UtcNow.AddHours(1))
        try {
            $trust = [Eitmad.Platform.Windows.Installation.PublisherCertificateTrust]::new(
                $other.RawData, $other.GetCertHashString([Security.Cryptography.HashAlgorithmName]::SHA256))
            try { Reject { $trust.VerifyInstallerSigner($executable) } } finally { $trust.Dispose() }
        } finally { $other.Dispose() }
    } finally { $rsa.Dispose() }
    Reject { Assert-PackageSignature $executable 'CN=Wrong publisher' }
    Assert-PackageSignature $executable $subject
    # A real signed catalog and anchor must use the same exact certificate.
    $payload = Join-Path $root 'payload'
    [void][IO.Directory]::CreateDirectory($payload)
    Copy-Item -LiteralPath $executable -Destination (Join-Path $payload 'EitmadSetup.exe')
    $catalog = Join-Path $payload 'offline.cat'
    New-FileCatalog -Path $payload -CatalogFilePath $catalog -CatalogVersion 2.0 | Out-Null
    & (Join-Path $PSScriptRoot 'sign-windows.ps1') -Path $catalog -Publisher $subject -SigningHook $hook
    $request = @{root=$payload; anchor=(Join-Path $payload 'EitmadSetup.exe'); publisher=$subject} | ConvertTo-Json -Compress
    $verify = Join-Path $PSScriptRoot 'verify-offline.ps1'
    $request | & pwsh.exe -NoProfile -NonInteractive -File $verify
    if ($LASTEXITCODE -ne 0) { throw 'Trusted self-signed offline catalog was rejected.' }
    [IO.File]::WriteAllText((Join-Path $payload 'unexpected.txt'), 'tampered payload')
    $request | & pwsh.exe -NoProfile -NonInteractive -File $verify 2>$null
    if ($LASTEXITCODE -eq 0) { throw 'Changed catalog content was accepted.' }
    $bytes = [IO.File]::ReadAllBytes($executable)
    $bytes[2] = $bytes[2] -bxor 1
    [IO.File]::WriteAllBytes($executable, $bytes)
    Reject { [Eitmad.Platform.Windows.Installation.AuthenticodeTrust]::Verify($executable) }
    Reject { Assert-PackageSignature $executable $subject }
    Write-Output 'Self-signed EXE, public certificate, native trust, catalog, and tamper checks passed without timestamps.'
} finally {
    # Remove only the certificate/key and files created by this invocation.
    if ($certificate) {
        $store = [Security.Cryptography.X509Certificates.X509Store]::new('Root', 'CurrentUser')
        try { $store.Open('ReadWrite'); $store.Remove($certificate) } finally { $store.Dispose() }
        Remove-Item -LiteralPath ('Cert:\CurrentUser\My\' + $certificate.Thumbprint) -DeleteKey
        $certificate.Dispose()
    }
    $env:EITMAD_SIGNING_CERTIFICATE_THUMBPRINT = $previousThumbprint
    $resolved = [IO.Path]::GetFullPath($root)
    if ([IO.Path]::GetDirectoryName($resolved) -cne [IO.Path]::GetTempPath().TrimEnd('\') -or
        (Get-Item -LiteralPath $resolved).Attributes -band [IO.FileAttributes]::ReparsePoint -or
        @(Get-ChildItem -LiteralPath $resolved -Recurse -Force | Where-Object { $_.Attributes -band [IO.FileAttributes]::ReparsePoint }).Count) {
        throw 'Temporary proof cleanup containment failed.'
    }
    Remove-Item -LiteralPath $resolved -Recurse -Force
}

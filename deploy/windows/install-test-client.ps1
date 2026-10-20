#Requires -Version 5.1
[CmdletBinding()]
param(
    [ValidateSet('Verify', 'Trust', 'Install')][string]$Action = 'Verify',
    [Parameter(Mandatory)][ValidatePattern('^[a-fA-F0-9]{64}$')][string]$ExpectedCertificateSha256,
    [Parameter(Mandatory)][ValidatePattern('^[a-fA-F0-9]{64}$')][string]$ExpectedPackageSha256
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$publisher = 'CN=Eitmad Local Test'
$certificatePath = Join-Path $PSScriptRoot 'Eitmad-Local-Test.cer'
$packagePath = Join-Path $PSScriptRoot 'Eitmad-Desktop-Test.msix'
foreach ($pair in @(@($certificatePath, $ExpectedCertificateSha256), @($packagePath, $ExpectedPackageSha256))) {
    if ((Get-FileHash -LiteralPath $pair[0] -Algorithm SHA256).Hash -ine $pair[1]) {
        throw 'Test certificate or package hash differs from the authenticated handover. Nothing was installed or trusted.'
    }
}
$certificate = New-Object Security.Cryptography.X509Certificates.X509Certificate2($certificatePath)
if ($certificate.Subject -cne $publisher -or $certificate.Issuer -cne $publisher -or $certificate.HasPrivateKey -or
    $certificate.NotBefore -gt (Get-Date) -or $certificate.NotAfter -le (Get-Date)) {
    throw 'Expected a valid public-only Eitmad Local Test certificate.'
}
$eku = @($certificate.Extensions | Where-Object { $_.Oid.Value -eq '2.5.29.37' })
$constraints = @($certificate.Extensions | Where-Object { $_.Oid.Value -eq '2.5.29.19' })
if ($eku.Count -ne 1 -or @($eku[0].EnhancedKeyUsages | Where-Object Value -eq '1.3.6.1.5.5.7.3.3').Count -ne 1 -or
    $eku[0].EnhancedKeyUsages.Count -ne 1 -or $constraints.Count -ne 1 -or $constraints[0].CertificateAuthority) {
    throw 'Test trust requires a code-signing-only leaf certificate, not a certificate authority.'
}
function Assert-TestSignature {
    $signature = Get-AuthenticodeSignature -LiteralPath $packagePath
    if ($signature.Status -ne 'Valid' -or -not $signature.SignerCertificate -or
        $signature.SignerCertificate.Thumbprint -cne $certificate.Thumbprint) {
        throw 'Test MSIX signature is not valid for the pinned certificate. Run Trust as Administrator first.'
    }
}
if ($Action -eq 'Trust') {
    $principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw 'Run Trust from PowerShell as Administrator. Install runs as the intended normal Windows user.'
    }
    $storePath = 'Cert:\LocalMachine\TrustedPeople\' + $certificate.Thumbprint
    $alreadyTrusted = Test-Path -LiteralPath $storePath
    if (-not $alreadyTrusted) {
        Import-Certificate -FilePath $certificatePath -CertStoreLocation 'Cert:\LocalMachine\TrustedPeople' | Out-Null
    }
    try { Assert-TestSignature } catch {
        if (-not $alreadyTrusted) { Remove-Item -LiteralPath $storePath }
        throw
    }
    Write-Host ('Test signing certificate trusted in LocalMachine\TrustedPeople: ' + $certificate.Thumbprint)
    Write-Host 'Close this Administrator window. Run Install as the intended normal Windows user.'
    return
}
Assert-TestSignature
if ($Action -eq 'Verify') { Write-Host 'PASS: test package hashes and pinned trusted MSIX signature.'; return }
Import-Module (Join-Path $PSScriptRoot 'package.psm1') -Force
Assert-WindowsPrerequisites
$installed = @(Get-AppxPackage -Name Eitmad.Desktop.Test)
if ($installed.Count) { throw 'The test app is already installed. Close it and remove only Eitmad.Desktop.Test before reinstalling; retain test runtime data.' }
Add-AppxPackage -Path $packagePath -ErrorAction Stop
$installed = @(Get-AppxPackage -Name Eitmad.Desktop.Test)
if ($installed.Count -ne 1 -or $installed[0].Publisher -cne $publisher -or $installed[0].Status -ne 'Ok') {
    throw 'The installed test identity was not confirmed. Inspect Windows AppXDeploymentServer events.'
}
Write-Host 'Test MSIX installed. Open the test app from Start and enter your own server connection and account.'
Write-Host 'Persistent test runtime: %LOCALAPPDATA%\Eitmad\TestClient\engine'

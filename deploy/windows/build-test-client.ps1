#Requires -Version 5.1
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$SourceArchive,
    [Parameter(Mandatory)][ValidatePattern('^[a-fA-F0-9]{64}$')][string]$ExpectedSourceSha256,
    [Parameter(Mandatory)][string]$WorkDirectory,
    [Parameter(Mandatory)][string]$OutputDirectory,
    [ValidatePattern('^[a-fA-F0-9]{40}$')][string]$CertificateThumbprint
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'package.psm1') -Force
if ((Get-FileHash -LiteralPath $SourceArchive -Algorithm SHA256).Hash -ine $ExpectedSourceSha256) {
    throw 'Source ZIP hash differs from the authenticated build record.'
}
$work = [IO.Path]::GetFullPath($WorkDirectory)
$output = [IO.Path]::GetFullPath($OutputDirectory)
if ((Test-Path -LiteralPath $work) -or (Test-Path -LiteralPath $output)) {
    throw 'Use new work and output directories. Existing files are preserved.'
}
$sdk = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
$tools = @(Get-ChildItem -LiteralPath $sdk -Directory | Where-Object Name -match '^10\.\d+\.\d+\.\d+$' |
    Sort-Object { [version]$_.Name } -Descending)
$makeappx = $null
$signtool = $null
foreach ($tool in $tools) {
    $make = Join-Path $tool.FullName 'x64\makeappx.exe'
    $sign = Join-Path $tool.FullName 'x64\signtool.exe'
    if ((Test-Path -LiteralPath $make) -and (Test-Path -LiteralPath $sign)) { $makeappx=$make; $signtool=$sign; break }
}
if (-not $makeappx) { throw 'Install Windows SDK x64 MakeAppx and SignTool first.' }
function Invoke-Checked([string]$File, [string[]]$Arguments) {
    $toolOutput = & $File @Arguments 2>&1
    if ($LASTEXITCODE -ne 0) {
        $toolOutput | Select-Object -Last 12 | Write-Host
        throw ('Test package tool failed: ' + [IO.Path]::GetFileName($File))
    }
}
[void][IO.Directory]::CreateDirectory($work)
[void][IO.Directory]::CreateDirectory($output)
$source = Join-Path $work 'source'
Expand-Archive -LiteralPath $SourceArchive -DestinationPath $source
$release = Read-EitmadPackage -Directory $source -Target windows-desktop -AllowUnsignedValidation `
    -ExpectedReleaseSha256 (Get-FileHash -LiteralPath (Join-Path $source 'release.json') -Algorithm SHA256).Hash
$packages = @(Get-ChildItem -LiteralPath $source -Filter '*.msix' -File)
if ($packages.Count -ne 1) { throw 'Expected exactly one source MSIX.' }
$payload = Join-Path $work 'payload'
Invoke-Checked $makeappx @('unpack', '/p', $packages[0].FullName, '/d', $payload)
# These are generated MSIX container files, not application payload entries.
foreach ($name in 'AppxBlockMap.xml', '[Content_Types].xml') {
    $path = Join-Path $payload $name
    if (Test-Path -LiteralPath $path) { [IO.File]::Delete($path) }
}
$original = Read-EitmadPackage -Directory $payload -Target windows-desktop -AllowUnsignedValidation `
    -ExpectedReleaseSha256 $release.payloadInventorySha256
$manifestPath = Join-Path $payload 'AppxManifest.xml'
[xml]$manifest = [IO.File]::ReadAllText($manifestPath)
if ($manifest.Package.Identity.Name -cne 'Eitmad.Desktop') { throw 'Expected the original desktop identity.' }
$publisher = 'CN=Eitmad Local Test'
$manifest.Package.Identity.Name = 'Eitmad.Desktop.Test'
$manifest.Package.Identity.Publisher = $publisher
$label = $manifest.Package.Properties.DisplayName + ' - ' + (-join ([char[]]@(0x627,0x62e,0x62a,0x628,0x627,0x631)))
$manifest.Package.Properties.DisplayName = $label
$application = $manifest.Package.Applications.Application
$application.Executable = 'Eitmad.TestLauncher.exe'
$application.VisualElements.DisplayName = $label
[IO.File]::WriteAllText($manifestPath, $manifest.OuterXml, (New-Object Text.UTF8Encoding($false)))
$compiler = Join-Path $env:SystemRoot 'Microsoft.NET\Framework64\v4.0.30319\csc.exe'
Invoke-Checked $compiler @('/nologo', '/target:winexe', '/optimize+', '/warnaserror+',
    ('/out:' + (Join-Path $payload 'Eitmad.TestLauncher.exe')), (Join-Path $PSScriptRoot 'TestClientLauncher.cs'))
$original.publisher = $publisher
$original.target = 'windows-desktop-test'
$original.productionEligible = $false
$original.files = @{}
Get-ChildItem -LiteralPath $payload -Recurse -File | Where-Object FullName -ne (Join-Path $payload 'release.json') | ForEach-Object {
    $relative = $_.FullName.Substring($payload.Length + 1).Replace('\', '/')
    $original.files[$relative] = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
}
$original | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath (Join-Path $payload 'release.json') -Encoding UTF8
$package = Join-Path $output 'Eitmad-Desktop-Test.msix'
Invoke-Checked $makeappx @('pack', '/d', $payload, '/p', $package)
if ($CertificateThumbprint) {
    $certificate = Get-Item -LiteralPath ('Cert:\CurrentUser\My\' + $CertificateThumbprint)
} else {
    $certificate = New-SelfSignedCertificate -Type Custom -Subject $publisher -FriendlyName 'Eitmad local MSIX testing only' `
        -CertStoreLocation 'Cert:\CurrentUser\My' -KeyAlgorithm RSA -KeyLength 3072 -HashAlgorithm SHA256 `
        -KeyExportPolicy NonExportable -KeyUsage DigitalSignature -NotAfter (Get-Date).AddYears(1) `
        -TextExtension @('2.5.29.37={text}1.3.6.1.5.5.7.3.3', '2.5.29.19={text}')
}
if ($certificate.Subject -cne $publisher -or -not $certificate.HasPrivateKey -or $certificate.NotAfter -le (Get-Date)) {
    throw 'Expected the retained Eitmad Local Test certificate with its private signing key.'
}
$publicCertificate = Join-Path $output 'Eitmad-Local-Test.cer'
Export-Certificate -Cert $certificate -FilePath $publicCertificate -Type CERT | Out-Null
Invoke-Checked $signtool @('sign', '/fd', 'SHA256', '/sha1', $certificate.Thumbprint, '/s', 'My', $package)
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'install-test-client.ps1') -Destination $output
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'package.psm1') -Destination $output
$record = [ordered]@{purpose='local-test-only'; productionEligible=$false; identity='Eitmad.Desktop.Test';
    publisher=$publisher; windowsVersion=$release.windowsVersion; certificateThumbprint=$certificate.Thumbprint;
    certificateSha256=(Get-FileHash -LiteralPath $publicCertificate -Algorithm SHA256).Hash.ToLowerInvariant();
    packageSha256=(Get-FileHash -LiteralPath $package -Algorithm SHA256).Hash.ToLowerInvariant();
    sourceArchiveSha256=$ExpectedSourceSha256.ToLowerInvariant(); sourceCommit=$release.sourceCommit;
    signingKeyStorage='Non-exported Windows CurrentUser My certificate key; never copy to clients';
    runtime='%LOCALAPPDATA%\Eitmad\TestClient\engine'; expiresAt=$certificate.NotAfter.ToUniversalTime().ToString('o')}
$record | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $output 'test-release.json') -Encoding UTF8
foreach ($entry in @(@('Trust-TestCertificate.cmd', 'Trust'), @('Install-TestApp.cmd', 'Install'))) {
    $command = 'powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0install-test-client.ps1" ' +
        $entry[1] + ' -ExpectedCertificateSha256 ' + $record.certificateSha256 +
        ' -ExpectedPackageSha256 ' + $record.packageSha256
    [IO.File]::WriteAllLines((Join-Path $output $entry[0]), @('@echo off', $command, 'pause'), [Text.Encoding]::ASCII)
}
$record | ConvertTo-Json

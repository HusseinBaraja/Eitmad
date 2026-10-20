#Requires -Version 5.1
[CmdletBinding()]
param([Parameter(Mandatory)][string]$BundleDirectory)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$record = Get-Content -LiteralPath (Join-Path $BundleDirectory 'test-release.json') -Raw | ConvertFrom-Json
$installer = Join-Path $BundleDirectory 'install-test-client.ps1'
function Reject([scriptblock]$Operation) {
    $rejected = $false
    try { & $Operation } catch {
        if ($_.Exception.Message -notlike '*hash differs*') { throw }
        $rejected = $true
    }
    if (-not $rejected) { throw 'Changed test certificate/package was accepted.' }
}
$trustBefore = @(Get-ChildItem Cert:\LocalMachine\TrustedPeople | ForEach-Object Thumbprint)
Reject { & $installer Trust -ExpectedCertificateSha256 ('0' * 64) -ExpectedPackageSha256 $record.packageSha256 }
Reject { & $installer Trust -ExpectedCertificateSha256 $record.certificateSha256 -ExpectedPackageSha256 ('0' * 64) }
$proof = Join-Path ([IO.Path]::GetTempPath()) ('eitmad-test-signing-proof-' + [Guid]::NewGuid().ToString('N'))
[void][IO.Directory]::CreateDirectory($proof)
Copy-Item -LiteralPath $installer, (Join-Path $BundleDirectory 'Eitmad-Local-Test.cer') -Destination $proof
$damagedPackage = Join-Path $proof 'Eitmad-Desktop-Test.msix'
[IO.File]::WriteAllText($damagedPackage, 'changed bytes')
Reject { & (Join-Path $proof 'install-test-client.ps1') Trust -ExpectedCertificateSha256 $record.certificateSha256 -ExpectedPackageSha256 $record.packageSha256 }
$trustAfter = @(Get-ChildItem Cert:\LocalMachine\TrustedPeople | ForEach-Object Thumbprint)
if (Compare-Object $trustBefore $trustAfter) { throw 'Rejected test inputs changed certificate trust.' }
& $installer Verify -ExpectedCertificateSha256 $record.certificateSha256 -ExpectedPackageSha256 $record.packageSha256
Write-Host 'PASS: pinned trusted signature, wrong-certificate rejection, wrong-package rejection, and changed-byte rejection before trust mutation.'

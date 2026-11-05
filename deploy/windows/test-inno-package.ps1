#Requires -Version 7.4
# Compile both real Inno templates with a small synthetic payload. Do not install.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Compiler,
    [Parameter(Mandatory)][string]$SigningCertificateThumbprint,
    [string]$OutputDirectory = (Join-Path ([IO.Path]::GetTempPath()) ('eitmad-inno-proof-' + [Guid]::NewGuid().ToString('N')))
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
if (Test-Path -LiteralPath $OutputDirectory) { throw 'Use a new evidence directory.' }
[void][IO.Directory]::CreateDirectory($OutputDirectory)
$previousCertificate = $env:EITMAD_SIGNING_CERTIFICATE_THUMBPRINT
$previousCompiler = $env:EITMAD_INNO_COMPILER
try {
    $env:EITMAD_SIGNING_CERTIFICATE_THUMBPRINT = $SigningCertificateThumbprint
    $env:EITMAD_INNO_COMPILER = $Compiler
    Import-Module (Join-Path $PSScriptRoot 'signing-certificate.psm1') -Force
    $certificate = Get-EitmadSigningCertificate
    $certificateFile = Join-Path $OutputDirectory 'publisher.cer'
    & (Join-Path $PSScriptRoot 'sign-with-self-signed-certificate.ps1') -ExportCertificate $certificateFile
    $payload = Join-Path $OutputDirectory 'payload'
    [void][IO.Directory]::CreateDirectory((Join-Path $payload 'setup'))
    Copy-Item -LiteralPath (Join-Path $root 'shells/windows-setup/bin/Release/net8.0-windows/EitmadSetup.exe') -Destination (Join-Path $payload 'setup/EitmadSetup.exe')
    [IO.File]::WriteAllText((Join-Path $payload 'SYNTHETIC-PAYLOAD.txt'), 'Compile and UI inspection only. This is not an installable Eitmad release.')
    [IO.File]::WriteAllText((Join-Path $payload 'release.json'), '{"synthetic":true}')
    $tools = Join-Path $payload 'server/application/deploy/windows'
    [void][IO.Directory]::CreateDirectory($tools)
    foreach ($name in 'uninstall.ps1', 'uninstallation.psm1', 'host.psm1', 'package.psm1', 'verify-offline.ps1') {
        Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination (Join-Path $tools $name)
    }
    foreach ($path in Get-ChildItem -LiteralPath $payload -Recurse -File | Where-Object { $_.Extension -in '.exe', '.ps1', '.psm1' }) {
        & (Join-Path $PSScriptRoot 'sign-windows.ps1') -Path $path.FullName -Publisher $certificate.Subject -SigningHook (Join-Path $PSScriptRoot 'sign-with-self-signed-certificate.ps1')
    }
    $env:EITMAD_INNO_PROOF_DIRECTORY = $OutputDirectory
    @'
import importlib.util
import os
from pathlib import Path
spec = importlib.util.spec_from_file_location('build_artifacts', 'scripts/release/build_artifacts.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
root = Path(os.environ['EITMAD_INNO_PROOF_DIRECTORY'])
for role in ('server', 'client'):
    executable = module.publish_installer(root / role, root / 'payload', role, '0.0.0', root / 'publisher.cer',
        'CN=Eitmad', module.ROOT / 'deploy/windows/sign-with-self-signed-certificate.ps1')
    print(executable)
'@ | python -
    if ($LASTEXITCODE -ne 0) { throw 'Inno package compilation failed.' }
    foreach ($role in 'server', 'client') {
        $exe = Join-Path $OutputDirectory ($role + '/' + $(if ($role -eq 'server') { 'EitmadServerSetup.exe' } else { 'EitmadClientSetup.exe' }))
        $signature = Get-AuthenticodeSignature -LiteralPath $exe
        if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Thumbprint -cne $certificate.Thumbprint) {
            throw 'Inno package signature differs from the retained publisher.'
        }
    }
    Write-Host ('Signed Inno package proof passed. Synthetic evidence: ' + $OutputDirectory)
} finally {
    $env:EITMAD_SIGNING_CERTIFICATE_THUMBPRINT = $previousCertificate
    $env:EITMAD_INNO_COMPILER = $previousCompiler
    Remove-Item Env:EITMAD_INNO_PROOF_DIRECTORY -ErrorAction SilentlyContinue
}

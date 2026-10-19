#Requires -Version 5.1
[CmdletBinding()]
param([Parameter(Mandatory)][string]$Path)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
# Only non-secret tool/configuration paths are accepted here. Authentication uses
# the signing node's managed/workload identity through DefaultAzureCredential.
foreach ($name in 'EITMAD_SIGNTOOL', 'EITMAD_SIGNING_DLIB', 'EITMAD_SIGNING_METADATA') {
    $value = [Environment]::GetEnvironmentVariable($name, 'Process')
    if (-not $value -or -not (Test-Path -LiteralPath $value -PathType Leaf)) {
        throw "Missing external signing prerequisite: $name"
    }
}
$metadata = Get-Content -LiteralPath $env:EITMAD_SIGNING_METADATA -Raw | ConvertFrom-Json
$endpoint = [Uri]$metadata.Endpoint
if ($endpoint.Scheme -ne 'https' -or $endpoint.DnsSafeHost -notmatch '^[a-z0-9]+\.codesigning\.azure\.net$' -or
    $endpoint.UserInfo -or $endpoint.Query -or $endpoint.Fragment -or
    -not $metadata.CodeSigningAccountName -or -not $metadata.CertificateProfileName) {
    throw 'Use the approved regional Artifact Signing endpoint and certificate profile metadata.'
}
foreach ($file in $env:EITMAD_SIGNTOOL, $env:EITMAD_SIGNING_DLIB) {
    if ((Get-AuthenticodeSignature -LiteralPath $file).Status -ne 'Valid') {
        throw 'Signing tools require a valid trusted vendor signature.'
    }
}
# HTTP is Microsoft's published TSA URI. The returned RFC3161 response is signed
# and independently verified; this is not the package distribution transport.
& $env:EITMAD_SIGNTOOL sign /fd SHA256 /tr http://timestamp.acs.microsoft.com /td SHA256 `
    /dlib $env:EITMAD_SIGNING_DLIB /dmdf $env:EITMAD_SIGNING_METADATA $Path
if ($LASTEXITCODE -ne 0) { throw 'Azure Artifact Signing failed. Check profile signer role, identity validation, and regional endpoint.' }

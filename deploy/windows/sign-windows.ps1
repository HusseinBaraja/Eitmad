#Requires -Version 5.1
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Path,
    [Parameter(Mandatory)][string]$Publisher,
    [Parameter(Mandatory)][string]$SigningHook
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if ($Publisher -eq 'CN=Eitmad Unsigned Validation') { throw 'Supply the approved production publisher identity before signing.' }
# The hook accepts one file path, signs with an external hardware-backed key and
# approved RFC3161 timestamp service, and throws on failure. Never accept a PFX here.
$files = if (Test-Path -LiteralPath $Path -PathType Container) {
    Get-ChildItem -LiteralPath $Path -Recurse -File | Where-Object Extension -In '.exe', '.dll', '.ps1', '.psm1', '.msix', '.cat'
} else { Get-Item -LiteralPath $Path }
foreach ($file in $files) {
    $signature = Get-AuthenticodeSignature -LiteralPath $file.FullName
    $owned = $file.Name -like 'Eitmad*' -or $file.Extension -in '.ps1', '.psm1', '.msix', '.cat'
    if ($signature.Status -ne 'Valid' -or ($owned -and $signature.SignerCertificate.Subject -cne $Publisher)) {
        & $SigningHook -Path $file.FullName
        if (-not $?) { throw 'External signing hook failed.' }
        $signature = Get-AuthenticodeSignature -LiteralPath $file.FullName
    }
    if ($signature.Status -ne 'Valid' -or -not $signature.TimeStamperCertificate -or
        ($owned -and $signature.SignerCertificate.Subject -cne $Publisher)) {
        throw 'Signing verification failed: require trusted Authenticode, timestamp, and the approved publisher.'
    }
}

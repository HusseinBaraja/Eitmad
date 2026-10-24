#Requires -Version 5.1
[CmdletBinding()]
param([Parameter(Mandatory)][string]$Directory)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetFullPath($Directory)
for ($parent = $root; $parent; $parent = Split-Path $parent -Parent) {
    if ((Get-Item -LiteralPath $parent -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) {
        throw 'Runtime inputs cannot use reparse points.'
    }
}
$required = @('concrt140.dll', 'msvcp140.dll', 'msvcp140_1.dll', 'msvcp140_2.dll',
    'msvcp140_atomic_wait.dll', 'msvcp140_codecvt_ids.dll', 'vccorlib140.dll', 'vcruntime140.dll', 'vcruntime140_1.dll')
$files = @(Get-ChildItem -LiteralPath $root -Force)
if ($files.Count -lt $required.Count -or $files.Count -gt 32) { throw 'Use the complete reviewed x64 Microsoft.VC143.CRT redistributable directory.' }
$versions = @()
$hashes = @{}
foreach ($file in $files) {
    if ($file.PSIsContainer -or ($file.Attributes -band [IO.FileAttributes]::ReparsePoint) -or
        $file.Name -notmatch '^(concrt140|msvcp140(?:_[a-z0-9_]+)?|vccorlib140|vcruntime140(?:_[a-z0-9_]+)?)\.dll$' -or
        $file.Length -gt 20MB) { throw 'Unexpected runtime input; use only the Microsoft CRT redistribution files.' }
    $stream = [IO.File]::OpenRead($file.FullName)
    $reader = New-Object IO.BinaryReader($stream)
    try {
        # Hold a read-only handle without write/delete sharing until its signature
        # and digest are recorded. The copy step must match that verified digest.
        $signature = Get-AuthenticodeSignature -LiteralPath $file.FullName
        if ($signature.Status -ne 'Valid' -or
            $signature.SignerCertificate.Subject -notmatch '(^|, )O=Microsoft Corporation(,|$)') {
            throw 'Runtime files require trusted Microsoft signatures.'
        }
        $version = [version]$file.VersionInfo.FileVersion
        if ($version -lt [version]'14.40.0.0') { throw 'Use a reviewed Microsoft CRT version 14.40 or later.' }
        $versions += $version.ToString()
        if ($reader.ReadUInt16() -ne 0x5a4d) { throw 'Runtime file is not a Windows executable.' }
        $stream.Position = 60
        $header = $reader.ReadUInt32()
        if ($header -gt $stream.Length - 6) { throw 'Invalid runtime PE header.' }
        $stream.Position = $header
        if ($reader.ReadUInt32() -ne 0x4550 -or $reader.ReadUInt16() -ne 0x8664) { throw 'Runtime files must target x64.' }
        $stream.Position = 0
        $hash = [Security.Cryptography.SHA256]::Create()
        try { $hashes[$file.Name] = [BitConverter]::ToString($hash.ComputeHash($stream)).Replace('-', '').ToLowerInvariant() }
        finally { $hash.Dispose() }
    } finally { $reader.Dispose(); $stream.Dispose() }
}
foreach ($name in $required) {
    if ($name -cnotin $files.Name) { throw 'The runtime directory is incomplete.' }
}
if (@($versions | Select-Object -Unique).Count -ne 1) { throw 'Do not mix Microsoft CRT versions.' }
[pscustomobject]@{version = $versions[0]; files = @($files.Name | Sort-Object); hashes = $hashes} | ConvertTo-Json

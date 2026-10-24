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
# Test-FileCatalog opens payload files with sharing incompatible with the immutable
# leases held by both launchers. Read its authenticated catalog inventory against
# an empty directory to authenticate release.json. That manifest binds raw SHA256
# hashes for every payload file (catalog PE hashes use Authenticode semantics).
# Compare every leased file with the authenticated manifest using read sharing.
# ValidationFailed is expected for that empty directory; it is never acceptance.
$empty = Join-Path ([IO.Path]::GetTempPath()) ('eitmad-catalog-' + [Guid]::NewGuid().ToString('N'))
[void][IO.Directory]::CreateDirectory($empty)
try {
    $result = Microsoft.PowerShell.Security\Test-FileCatalog -Path $empty -CatalogFilePath $catalog -Detailed
} finally { [IO.Directory]::Delete($empty) }
if ($result.HashAlgorithm -ne 'SHA256' -or $result.CatalogItems.Count -eq 0) {
    throw 'Offline catalog has no SHA256 inventory.'
}
$root = [IO.Path]::GetFullPath($inputData.root).TrimEnd('\') + '\'
$manifestPath = Join-Path $root 'release.json'
if (-not $result.CatalogItems.ContainsKey('release.json')) {
    throw 'Offline catalog does not authenticate the release inventory.'
}
$manifestStream = [IO.FileStream]::new($manifestPath, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
$manifestHash = [Security.Cryptography.SHA256]::Create()
try {
    $hash = [BitConverter]::ToString($manifestHash.ComputeHash($manifestStream)).Replace('-', '')
    if ($hash -cne $result.CatalogItems['release.json'].ToUpperInvariant()) {
        throw 'Offline release inventory differs from its signed catalog.'
    }
    $manifestStream.Position = 0
    $reader = [IO.StreamReader]::new($manifestStream, [Text.Encoding]::UTF8)
    try { $manifest = $reader.ReadToEnd() | ConvertFrom-Json }
    finally { $reader.Dispose() }
} finally { $manifestStream.Dispose(); $manifestHash.Dispose() }
$files = [Collections.Generic.Dictionary[string,string]]::new([StringComparer]::OrdinalIgnoreCase)
foreach ($entry in $manifest.files.PSObject.Properties) {
    if ($entry.Name -in @('release.json', 'offline.cat') -or $entry.Value -cnotmatch '^[a-f0-9]{64}$') {
        throw 'Offline release inventory has an invalid file entry.'
    }
    $files.Add($entry.Name, $entry.Value)
}
if ($manifest.schemaVersion -ne 1 -or $files.Count -eq 0 -or $result.CatalogItems.Count -ne $files.Count + 1) {
    throw 'Offline release inventory is incomplete.'
}
$directories = [Collections.Generic.Stack[string]]::new()
$directories.Push($root)
$count = 0
$sha256 = [Security.Cryptography.SHA256]::Create()
try {
    while ($directories.Count -gt 0) {
        foreach ($path in [IO.Directory]::EnumerateFileSystemEntries($directories.Pop())) {
            $attributes = [IO.File]::GetAttributes($path)
            if (($attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw 'Offline payload cannot use reparse points.'
            }
            if (($attributes -band [IO.FileAttributes]::Directory) -ne 0) {
                $directories.Push($path)
                continue
            }
            $relative = $path.Substring($root.Length).Replace('\', '/')
            if ($relative -iin @('offline.cat', 'release.json')) { continue }
            if (-not $files.ContainsKey($relative)) {
                throw 'Offline payload contains an unexpected file.'
            }
            $stream = [IO.FileStream]::new($path, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
            try { $hash = [BitConverter]::ToString($sha256.ComputeHash($stream)).Replace('-', '') }
            finally { $stream.Dispose() }
            if ($hash -cne $files[$relative].ToUpperInvariant()) {
                throw 'Offline payload contains a changed file.'
            }
            $count++
        }
    }
} finally { $sha256.Dispose() }
if ($count -ne $files.Count) {
    throw 'Offline payload contains missing files.'
}

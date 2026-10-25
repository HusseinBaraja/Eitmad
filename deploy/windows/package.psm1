Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Assert-PackageSignature([string]$Path, [string]$Publisher) {
    $signature = Get-AuthenticodeSignature -LiteralPath $Path
    if ($signature.Status -ne 'Valid') {
        throw 'A trusted signature is required. Unsigned artifacts are validation-only.'
    }
    $file = Get-Item -LiteralPath $Path
    if ($file.Name -like 'Eitmad*' -or $file.Extension -in '.ps1', '.psm1', '.msix', '.cat') {
        $certificate = $signature.SignerCertificate
        $eku = @($certificate.Extensions | Where-Object { $_.Oid.Value -eq '2.5.29.37' })
        $constraints = @($certificate.Extensions | Where-Object { $_.Oid.Value -eq '2.5.29.19' })
        if ($certificate.Subject -cne $Publisher -or $certificate.Subject -cne $certificate.Issuer -or
            $certificate.NotBefore -gt (Get-Date) -or $certificate.NotAfter -le (Get-Date) -or
            $eku.Count -ne 1 -or $eku[0].EnhancedKeyUsages.Count -ne 1 -or
            $eku[0].EnhancedKeyUsages[0].Value -cne '1.3.6.1.5.5.7.3.3' -or
            $constraints.Count -ne 1 -or $constraints[0].CertificateAuthority) {
            throw 'Package publisher must match a valid self-signed code-signing-only leaf certificate.'
        }
    }
}

function Read-EitmadPackage {
    param([string]$Directory, [string]$Target,
        [Parameter(Mandatory)][ValidatePattern('^[a-fA-F0-9]{64}$')][string]$ExpectedReleaseSha256,
        [string]$Publisher, [switch]$AllowUnsignedValidation)
    $root = [IO.Path]::GetFullPath($Directory).TrimEnd('\')
    $manifest = Join-Path $root 'release.json'
    if ((Get-FileHash -LiteralPath $manifest -Algorithm SHA256).Hash -ine $ExpectedReleaseSha256) {
        throw 'Release inventory digest differs from the authenticated release record.'
    }
    $release = Get-Content -LiteralPath $manifest -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($release.schemaVersion -ne 1 -or $release.target -cne $Target -or $release.architecture -cne 'x64' -or
        $release.windowsVersion -notmatch '^\d+\.\d+\.\d+\.0$') { throw 'Unsupported release inventory.' }
    if (-not $AllowUnsignedValidation -and (-not $Publisher -or $release.publisher -cne $Publisher)) {
        throw 'Supply the approved publisher from the authenticated release record.'
    }
    for ($parent = $root; $parent; $parent = Split-Path $parent -Parent) {
        if ((Get-Item -LiteralPath $parent -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) {
            throw 'Package paths cannot use reparse points.'
        }
    }
    $files = @(Get-ChildItem -LiteralPath $root -Force -Recurse)
    if (@($files | Where-Object { $_.Attributes -band [IO.FileAttributes]::ReparsePoint }).Count) {
        throw 'Package inputs cannot use reparse points.'
    }
    foreach ($entry in $release.files.PSObject.Properties) {
        if ($entry.Name -match '(^/|\\|:|(^|/)\.\.(/|$))') { throw 'Unsafe package inventory path.' }
        $file = Join-Path $root $entry.Name
        if ((Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash -ine $entry.Value) {
            throw 'Package payload differs from the release inventory. No installation was changed.'
        }
        if (-not $AllowUnsignedValidation -and [IO.Path]::GetExtension($file) -in '.exe', '.dll', '.ps1', '.psm1', '.msix') {
            Assert-PackageSignature $file $Publisher
        }
    }
    if (@($files | Where-Object { -not $_.PSIsContainer }).Count -ne @($release.files.PSObject.Properties).Count + 1) {
        throw 'The package contains unlisted files.'
    }
    return $release
}

function Assert-WindowsPrerequisites([switch]$PlatformOnly) {
    if (-not [Environment]::Is64BitOperatingSystem -or -not [Environment]::Is64BitProcess -or
        [Environment]::OSVersion.Version -lt [version]'10.0.19041.0') { throw 'Use x64 Windows build 19041 or later with x64 PowerShell.' }
    if ($PlatformOnly) { return }
    $vc = Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64' -ErrorAction SilentlyContinue
    if (-not $vc -or $vc.Installed -ne 1 -or [version]$vc.Version.TrimStart('v') -lt [version]'14.40.0.0') {
        throw 'Install the supported Microsoft Visual C++ 2015-2022 x64 Redistributable (14.40 or later), then retry.'
    }
}

function Assert-DesktopRuntime([string]$Package, [string]$ExpectedPayloadSha256) {
    # The caller authenticates the outer inventory and MSIX hash first. This check
    # proves the offline runtime is inside that payload, without registering it.
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = [IO.Compression.ZipFile]::OpenRead($Package)
    function Read-Entry([string]$Name) {
        $matches = @($archive.Entries | Where-Object FullName -CEQ $Name)
        if ($matches.Count -ne 1 -or $matches[0].Length -gt 20MB) { throw 'Desktop runtime payload is missing, ambiguous, or oversized.' }
        $stream = $matches[0].Open()
        $buffer = New-Object IO.MemoryStream
        try { $stream.CopyTo($buffer); return ,$buffer.ToArray() }
        finally { $buffer.Dispose(); $stream.Dispose() }
    }
    function Digest([byte[]]$Bytes) {
        $hash = [Security.Cryptography.SHA256]::Create()
        try { return [BitConverter]::ToString($hash.ComputeHash($Bytes)).Replace('-', '').ToLowerInvariant() }
        finally { $hash.Dispose() }
    }
    try {
        $payload = Read-Entry 'release.json'
        if (-not $ExpectedPayloadSha256 -or (Digest $payload) -ine $ExpectedPayloadSha256) { throw 'Desktop payload inventory differs from the authenticated release.' }
        $inventory = [Text.Encoding]::UTF8.GetString($payload).TrimStart([char]0xfeff) | ConvertFrom-Json
        $metadata = Read-Entry 'visual-cpp-runtime.json'
        if ((Digest $metadata) -ine $inventory.files.'visual-cpp-runtime.json') { throw 'Desktop runtime metadata differs from the payload inventory.' }
        $runtime = [Text.Encoding]::UTF8.GetString($metadata).TrimStart([char]0xfeff) | ConvertFrom-Json
        if ($runtime.architecture -cne 'x64' -or $runtime.deployment -cne 'app-local' -or
            [version]$runtime.version -lt [version]'14.40.0.0') { throw 'Unsupported desktop runtime.' }
        foreach ($name in 'concrt140.dll', 'msvcp140.dll', 'msvcp140_1.dll', 'msvcp140_2.dll',
            'msvcp140_atomic_wait.dll', 'msvcp140_codecvt_ids.dll', 'vccorlib140.dll', 'vcruntime140.dll', 'vcruntime140_1.dll') {
            $digest = Digest (Read-Entry $name)
            if ($digest -ine $runtime.files.$name -or $digest -ine $inventory.files.$name) { throw 'Desktop runtime is incomplete or differs from its inventory.' }
        }
        foreach ($name in 'coreclr.dll', 'hostfxr.dll', 'hostpolicy.dll', 'PresentationFramework.dll',
            'Eitmad.WindowsShell.runtimeconfig.json', 'AppxManifest.xml') {
            if ((Digest (Read-Entry $name)) -ine $inventory.files.$name) { throw 'Desktop offline runtime differs from its inventory.' }
        }
        $options = ([Text.Encoding]::UTF8.GetString((Read-Entry 'Eitmad.WindowsShell.runtimeconfig.json')) |
            ConvertFrom-Json).runtimeOptions
        if ($options.PSObject.Properties['framework'] -or $options.PSObject.Properties['frameworks'] -or
            -not $options.PSObject.Properties['includedFrameworks'] -or
            'Microsoft.NETCore.App' -notin $options.includedFrameworks.name -or
            'Microsoft.WindowsDesktop.App' -notin $options.includedFrameworks.name) {
            throw 'Desktop requires self-contained .NET and Windows Desktop runtimes.'
        }
        $manifest = [xml][Text.Encoding]::UTF8.GetString((Read-Entry 'AppxManifest.xml'))
        $namespaces = New-Object Xml.XmlNamespaceManager($manifest.NameTable)
        $namespaces.AddNamespace('m', 'http://schemas.microsoft.com/appx/manifest/foundation/windows10')
        foreach ($dependency in $manifest.SelectNodes('/m:Package/m:Dependencies/*', $namespaces)) {
            if ($dependency.LocalName -cne 'TargetDeviceFamily' -or
                $dependency.NamespaceURI -cne $namespaces.LookupNamespace('m')) {
                throw 'Offline desktop cannot require external package dependencies.'
            }
        }
        foreach ($entry in $archive.Entries) {
            if ($entry.Name -match '^(eitmad-server|Eitmad\.LanCertificate|postgres|initdb|psql|pg_[a-z_]+|createdb|createuser|dropdb|dropuser|pwsh|winsw[^.]*|WinSW\.NET461|host-service)\.exe$') {
                throw 'Client cannot include server components.'
            }
        }
    } finally { $archive.Dispose() }
}

function Assert-AppendOnlyMigrations($Previous, $Next) {
    foreach ($entry in $Previous.migrations.PSObject.Properties) {
        $current = $Next.migrations.PSObject.Properties[$entry.Name]
        if (-not $current -or $current.Value -cne $entry.Value) {
            throw 'A recorded migration was removed or changed. Stop and review the release; no automatic downgrade is supported.'
        }
    }
}

Export-ModuleMember -Function Read-EitmadPackage, Assert-PackageSignature, Assert-WindowsPrerequisites, Assert-DesktopRuntime, Assert-AppendOnlyMigrations

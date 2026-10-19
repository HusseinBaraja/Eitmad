Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Assert-PackageSignature([string]$Path, [string]$Publisher) {
    $signature = Get-AuthenticodeSignature -LiteralPath $Path
    if ($signature.Status -ne 'Valid' -or -not $signature.TimeStamperCertificate) {
        throw 'A trusted, timestamped signature is required. Unsigned artifacts are validation-only.'
    }
    $file = Get-Item -LiteralPath $Path
    if (($file.Name -like 'Eitmad*' -or $file.Extension -in '.ps1', '.psm1', '.msix') -and
        $signature.SignerCertificate.Subject -cne $Publisher) { throw 'Package publisher does not match the approved identity.' }
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

function Assert-WindowsPrerequisites {
    if (-not [Environment]::Is64BitOperatingSystem -or -not [Environment]::Is64BitProcess -or
        [Environment]::OSVersion.Version -lt [version]'10.0.19041.0') { throw 'Use x64 Windows build 19041 or later with x64 PowerShell.' }
    $vc = Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64' -ErrorAction SilentlyContinue
    if (-not $vc -or $vc.Installed -ne 1 -or [version]$vc.Version.TrimStart('v') -lt [version]'14.40.0.0') {
        throw 'Install the supported Microsoft Visual C++ 2015-2022 x64 Redistributable (14.40 or later), then retry.'
    }
}

function Assert-AppendOnlyMigrations($Previous, $Next) {
    foreach ($entry in $Previous.migrations.PSObject.Properties) {
        $current = $Next.migrations.PSObject.Properties[$entry.Name]
        if (-not $current -or $current.Value -cne $entry.Value) {
            throw 'A recorded migration was removed or changed. Stop and review the release; no automatic downgrade is supported.'
        }
    }
}

Export-ModuleMember -Function Read-EitmadPackage, Assert-PackageSignature, Assert-WindowsPrerequisites, Assert-AppendOnlyMigrations

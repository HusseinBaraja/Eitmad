#Requires -Version 5.1
[CmdletBinding()]
param()
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'package.psm1') -Force
$root = Join-Path ([IO.Path]::GetTempPath()) ('eitmad-package-proof-' + [Guid]::NewGuid().ToString('N'))
[void][IO.Directory]::CreateDirectory($root)
function Reject([scriptblock]$Operation, [string]$Message) {
    $rejected = $false
    try { & $Operation } catch { $rejected = $true; if ($_.Exception.Message -notlike "*$Message*") { throw } }
    if (-not $rejected) { throw "Expected failure: $Message" }
}
$binary = Join-Path $root 'eitmad-server.exe'
[IO.File]::WriteAllText($binary, 'synthetic executable')
$release = [pscustomobject]@{schemaVersion=1; target='windows-server'; architecture='x64'; windowsVersion='1.0.0.0';
    publisher='CN=Validation'; files=@{'eitmad-server.exe'=(Get-FileHash $binary -Algorithm SHA256).Hash}}
$manifest = Join-Path $root 'release.json'
function Save-Inventory {
    $release | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $manifest -Encoding UTF8
    $script:digest = (Get-FileHash $manifest -Algorithm SHA256).Hash
}
function Read-Inventory([switch]$Unsigned) {
    Read-EitmadPackage -Directory $root -Target windows-server -ExpectedReleaseSha256 $script:digest -Publisher 'CN=Validation' -AllowUnsignedValidation:$Unsigned
}
Save-Inventory
[void](Read-Inventory -Unsigned)
Reject { Read-EitmadPackage -Directory $root -Target windows-server -ExpectedReleaseSha256 $script:digest -Publisher 'CN=Other' } 'approved publisher'
Reject { Read-Inventory } 'trusted, timestamped'
[IO.File]::WriteAllText($binary, 'changed executable')
Reject { Read-Inventory -Unsigned } 'differs from the release inventory'
[IO.File]::WriteAllText($binary, 'synthetic executable')
[IO.File]::WriteAllText((Join-Path $root 'unexpected.db'), 'must not ship')
Reject { Read-Inventory -Unsigned } 'unlisted files'
Remove-Item -LiteralPath (Join-Path $root 'unexpected.db')
$release.files = @{'../outside.exe'='00'}
Save-Inventory
Reject { Read-Inventory -Unsigned } 'Unsafe package inventory path'
$previous = [pscustomobject]@{migrations=[pscustomobject]@{one='immutable'}}
$next = [pscustomobject]@{migrations=[pscustomobject]@{one='immutable'; two='added'}}
Assert-AppendOnlyMigrations $previous $next
$next.migrations.one = 'changed'
Reject { Assert-AppendOnlyMigrations $previous $next } 'migration was removed or changed'
$next.migrations = [pscustomobject]@{two='added'}
Reject { Assert-AppendOnlyMigrations $previous $next } 'migration was removed or changed'
Add-Type -AssemblyName System.IO.Compression.FileSystem
$desktop = Join-Path $root 'desktop'
[void][IO.Directory]::CreateDirectory($desktop)
$runtimeFiles = @{}
foreach ($name in 'concrt140.dll', 'msvcp140.dll', 'msvcp140_1.dll', 'msvcp140_2.dll',
    'msvcp140_atomic_wait.dll', 'msvcp140_codecvt_ids.dll', 'vccorlib140.dll', 'vcruntime140.dll', 'vcruntime140_1.dll') {
    $path = Join-Path $desktop $name
    [IO.File]::WriteAllText($path, 'synthetic runtime: ' + $name)
    $runtimeFiles[$name] = (Get-FileHash $path -Algorithm SHA256).Hash
}
$runtime = [pscustomobject]@{version='14.44.35211.0'; architecture='x64'; deployment='app-local'; files=$runtimeFiles.Clone()}
$runtimePath = Join-Path $desktop 'visual-cpp-runtime.json'
$runtime | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $runtimePath -Encoding UTF8
$runtimeFiles['visual-cpp-runtime.json'] = (Get-FileHash $runtimePath -Algorithm SHA256).Hash
foreach ($name in 'coreclr.dll', 'hostfxr.dll', 'hostpolicy.dll', 'PresentationFramework.dll') {
    [IO.File]::WriteAllText((Join-Path $desktop $name), 'synthetic offline runtime')
    $runtimeFiles[$name] = (Get-FileHash (Join-Path $desktop $name)).Hash
}
$configuration = Join-Path $desktop 'Eitmad.WindowsShell.runtimeconfig.json'
[IO.File]::WriteAllText($configuration, '{"runtimeOptions":{"includedFrameworks":[{"name":"Microsoft.NETCore.App","version":"8.0.0"},{"name":"Microsoft.WindowsDesktop.App","version":"8.0.0"}]}}')
$manifest = Join-Path $desktop 'AppxManifest.xml'
[IO.File]::WriteAllText($manifest, '<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"><Dependencies><TargetDeviceFamily Name="Windows.Desktop" /></Dependencies></Package>')
foreach ($path in $configuration, $manifest) { $runtimeFiles[(Split-Path $path -Leaf)] = (Get-FileHash $path).Hash }
$payloadPath = Join-Path $desktop 'release.json'
[pscustomobject]@{files=$runtimeFiles} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $payloadPath -Encoding UTF8
$payloadDigest = (Get-FileHash $payloadPath -Algorithm SHA256).Hash
function Pack-Desktop([string]$Name) {
    $package = Join-Path $root ($Name + '.msix')
    [IO.Compression.ZipFile]::CreateFromDirectory($desktop, $package)
    return $package
}
$complete = Pack-Desktop 'complete'
Assert-DesktopRuntime $complete $payloadDigest
foreach ($scenario in 'dependency', 'server-tool', 'framework-dependent') {
    if ($scenario -eq 'dependency') {
        [IO.File]::WriteAllText($manifest, '<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"><Dependencies><PackageDependency Name="External.Runtime" /></Dependencies></Package>')
    } elseif ($scenario -eq 'server-tool') {
        [IO.File]::WriteAllText((Join-Path $desktop 'pg_dump.exe'), 'synthetic server tool')
    } else {
        [IO.File]::WriteAllText($configuration, '{"runtimeOptions":{"framework":{"name":"Microsoft.NETCore.App","version":"8.0.0"}}}')
    }
    $policyFiles = $runtimeFiles.Clone()
    foreach ($path in $manifest, $configuration) { $policyFiles[(Split-Path $path -Leaf)] = (Get-FileHash $path).Hash }
    if ($scenario -eq 'server-tool') { $policyFiles['pg_dump.exe'] = (Get-FileHash (Join-Path $desktop 'pg_dump.exe')).Hash }
    [pscustomobject]@{files=$policyFiles} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $payloadPath -Encoding UTF8
    $policyDigest = (Get-FileHash $payloadPath).Hash
    $policyPackage = Pack-Desktop $scenario
    $reason = switch ($scenario) { 'dependency' { 'external package dependencies' }; 'server-tool' { 'server components' }; default { 'self-contained' } }
    Reject { Assert-DesktopRuntime $policyPackage $policyDigest } $reason
    # Restore the same synthetic complete input for the next independent case.
    [IO.File]::WriteAllText($manifest, '<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"><Dependencies><TargetDeviceFamily Name="Windows.Desktop" /></Dependencies></Package>')
    Remove-Item -LiteralPath (Join-Path $desktop 'pg_dump.exe') -ErrorAction SilentlyContinue
}
[pscustomobject]@{files=$runtimeFiles} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $payloadPath -Encoding UTF8
Reject { Assert-DesktopRuntime $complete ('0' * 64) } 'inventory differs'
[IO.File]::WriteAllText((Join-Path $desktop 'msvcp140.dll'), 'changed runtime')
$changed = Pack-Desktop 'changed'
Reject { Assert-DesktopRuntime $changed $payloadDigest } 'incomplete or differs'
[IO.File]::WriteAllText((Join-Path $desktop 'msvcp140.dll'), 'synthetic runtime: msvcp140.dll')
Remove-Item -LiteralPath (Join-Path $desktop 'vcruntime140_1.dll')
$missing = Pack-Desktop 'missing'
Reject { Assert-DesktopRuntime $missing $payloadDigest } 'missing, ambiguous'
& (Get-Module package) {
    function Get-ItemProperty { throw 'The per-user desktop queried the machine-wide runtime registry.' }
    Assert-WindowsPrerequisites -PlatformOnly
}
Write-Host 'PASS: package inventory, tamper/extra-file/path rejection, unsigned refusal, and append-only migration gate.'
Write-Host 'PASS: inventoried app-local desktop runtime, altered/missing DLL rejection, and no global runtime requirement.'
Write-Host "Synthetic evidence: $root"

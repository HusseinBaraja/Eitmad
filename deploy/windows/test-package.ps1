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
Write-Host 'PASS: package inventory, tamper/extra-file/path rejection, unsigned refusal, and append-only migration gate.'
Write-Host "Synthetic evidence: $root"

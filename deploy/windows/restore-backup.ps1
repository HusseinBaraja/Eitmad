#Requires -Version 5.1
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Generation,
    [Parameter(Mandatory)][string]$PostgresBin,
    [string]$StageDirectory
)
# Run after Windows image recovery, with the original installations disconnected.
# This command validates the cohort and optionally stages PostgreSQL into a NEW directory.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'backup.psm1') -Force
$module = Get-Module backup
try {
    $generationPath = (Get-Item -LiteralPath $Generation -Force).FullName.TrimEnd('\')
    if ((Split-Path $generationPath -Leaf) -notmatch '^\d{8}T\d{6}Z-[a-f0-9]{32}$') { throw 'Invalid generation.' }
    foreach ($item in @((Get-Item -LiteralPath $generationPath -Force)) + @(Get-ChildItem -LiteralPath $generationPath -Force -Recurse)) {
        if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Recovery generation contains an external link.' }
    }
    $marker = Get-Content -LiteralPath (Join-Path $generationPath 'complete.json') -Raw | ConvertFrom-Json
    if ($marker.version -ne 1 -or $marker.status -cne 'complete' -or $marker.id -cne (Split-Path $generationPath -Leaf) -or
        $marker.deploymentId -cne (Split-Path (Split-Path $generationPath -Parent) -Leaf)) { throw 'Generation identity mismatch.' }
    # Validate every participant before constructing any path from the manifest.
    $servers = @($marker.nodes | Where-Object {
        if ($_.id -notmatch '^[a-z][a-z0-9-]{0,31}$') { throw 'Unsafe participant id.' }
        $_.kind -eq 'server'
    })
    if ($servers.Count -ne 1) { throw 'Expected exactly one complete PostgreSQL archive.' }
    $cluster = Join-Path (Join-Path $generationPath $servers[0].id) 'cluster'
    if ((Get-FileHash -LiteralPath (Join-Path $cluster 'backup_manifest') -Algorithm SHA256).Hash -cne $marker.postgresManifestSha256) {
        throw 'PostgreSQL manifest identity mismatch.'
    }
    & (Join-Path $PostgresBin 'pg_verifybackup.exe') --exit-on-error $cluster *> $null
    if ($LASTEXITCODE -ne 0) { throw 'PostgreSQL backup verification failed.' }
    foreach ($node in $marker.nodes) {
        if ($node.computer -notmatch '^[A-Za-z0-9-]+$') { throw 'Invalid image computer name.' }
        $image = Join-Path (Join-Path $generationPath $node.id) 'image'
        & "$env:SystemRoot\System32\wbadmin.exe" get versions "-backupTarget:$image" "-machine:$($node.computer)" *> $null
        if ($LASTEXITCODE -ne 0) { throw 'A Windows image catalog is unavailable.' }
    }
    if ($StageDirectory) {
        Import-Module (Join-Path $PSScriptRoot 'host.psm1') -Force
        $hostModule = Get-Module host
        $stage = & $hostModule { param($Path) Get-SafePath $Path } $StageDirectory
        if (Test-Path -LiteralPath $stage) { throw 'Staging requires a new directory. Existing data was retained.' }
        & $hostModule { param($Path) Protect-Directory $Path } $stage
        foreach ($item in Get-ChildItem -LiteralPath $cluster -Force) { Copy-Item -LiteralPath $item.FullName -Destination $stage -Recurse }
        & (Join-Path $PostgresBin 'pg_verifybackup.exe') --exit-on-error $stage *> $null
        if ($LASTEXITCODE -ne 0) { throw 'Staged backup verification failed. Preserve the staged files.' }
    }
    Write-Host 'Complete cohort and PostgreSQL backup verified. Staging does not activate an installation.'
} catch { Write-Error 'eitmad.backup.restore-failed.v1'; exit 1 }

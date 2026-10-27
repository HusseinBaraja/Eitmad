#Requires -Version 5.1
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateSet('Initialize', 'Stop', 'Inspect')][string]$Action,
    [Parameter(Mandatory)][string]$StateDirectory,
    [Parameter(Mandatory)][string]$PostgresBin,
    [Parameter(Mandatory)][string]$CertificateExecutable,
    [Parameter(Mandatory)][string]$ServerName,
    [ValidateRange(1024, 65535)][int]$DatabasePort
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = New-Object Text.UTF8Encoding($false)
trap {
    # One bounded line can be read without waiting for inherited PostgreSQL handles.
    $diagnostic = [ordered]@{message=$_.Exception.Message; errorId=$_.FullyQualifiedErrorId;
        exceptionType=$_.Exception.GetType().FullName; script=$_.InvocationInfo.ScriptName;
        line=$_.InvocationInfo.ScriptLineNumber; stack=$_.ScriptStackTrace}
    $text = $diagnostic | ConvertTo-Json -Compress
    if ($text.Length -gt 2500) { $text = $text.Substring(0,2500) }
    [Console]::Error.WriteLine($text)
    exit 1
}
Import-Module (Join-Path $PSScriptRoot 'host.psm1') -Force

function Invoke-PrivateTool([string]$File, [string[]]$Arguments, [string]$InputText) {
    # This adapter owns Windows argv and process handles. No child output is published.
    $quoted = @($Arguments | ForEach-Object {
        '"' + [regex]::Replace([regex]::Replace($_, '(\\*)"', '$1$1\"'), '(\\+)$', '$1$1') + '"'
    })
    $start = New-Object Diagnostics.ProcessStartInfo
    $start.FileName = $File
    $start.Arguments = $quoted -join ' '
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardInput = $true
    # Do not wait for child output here. PostgreSQL can inherit these handles;
    # Rust reads only the bounded failure line and never waits for stderr EOF.
    $start.RedirectStandardOutput = $false
    $start.RedirectStandardError = $false
    $process = [Diagnostics.Process]::Start($start)
    try {
        if ($InputText) { $process.StandardInput.WriteLine($InputText) }
        $process.StandardInput.Close()
        if (-not $process.WaitForExit(90000)) {
            $process.Kill()
            [void]$process.WaitForExit(10000)
            throw 'The setup tool exceeded its time limit. Preserve the new state for inspection.'
        }
        if ($process.ExitCode -ne 0) { throw 'The setup tool failed. Its private output was withheld.' }
    } finally { $process.Dispose() }
}

# Reuse the host adapter's path policy without exposing another public installation API.
$hostAdapter = Get-Module host
$root = & $hostAdapter { param($path) Get-SafePath $path } $StateDirectory
$bin = & $hostAdapter { param($path) Get-SafePath $path } $PostgresBin
$cluster = Join-Path $root 'cluster'
if ($Action -eq 'Inspect') {
    & $hostAdapter { param($path) Assert-PlainTree $path; Assert-ProtectedParents $path } $root
    foreach ($relative in 'prepared-workshop.json', 'server.json', 'active-tls.json', 'trusted-ca.pem',
        'certificate-password', 'postgres-admin-password', 'application-password', 'token-key', 'cluster/PG_VERSION') {
        if (-not (Test-Path -LiteralPath (Join-Path $root $relative) -PathType Leaf)) { throw 'Preparation is incomplete. Preserve all files for recovery.' }
    }
    if ((Get-Content -LiteralPath (Join-Path $cluster 'PG_VERSION') -Raw).Trim() -ne '17' -or
        (Test-Path -LiteralPath (Join-Path $cluster 'postmaster.pid'))) { throw 'Preparation requires a complete stopped PostgreSQL 17 cluster.' }
    return
}
if ($Action -eq 'Stop') {
    if (Test-Path -LiteralPath (Join-Path $cluster 'postmaster.pid')) {
        & $hostAdapter { param($path) Assert-PlainTree $path } $root
        Invoke-PrivateTool (Join-Path $bin 'pg_ctl.exe') @('-D', $cluster, '-m', 'fast', '-w', 'stop')
    }
    return
}
if (Test-Path -LiteralPath $root) { throw 'Fresh setup requires a new directory. Existing state was not changed.' }
& $hostAdapter { param($path) Assert-PlainTree $path } (Split-Path $bin -Parent)
if ((& (Join-Path $bin 'pg_ctl.exe') --version) -notmatch '^pg_ctl \(PostgreSQL\) 17\.') {
    throw 'The reviewed PostgreSQL 17 distribution is required.'
}
if (@(Get-NetTCPConnection -State Listen -LocalPort $DatabasePort -ErrorAction SilentlyContinue).Count) {
    throw 'The database port is already used. No existing listener was changed.'
}
$inputText = [Console]::In.ReadToEnd()
if ($inputText.Length -gt 1024) { throw 'Invalid private setup input.' }
$secrets = $inputText | ConvertFrom-Json
$inputText = $null
if ($secrets.adminPassword -notmatch '^[A-Za-z0-9_-]{43}$' -or
    $secrets.certificatePassword -notmatch '^[A-Za-z0-9_-]{43}$') { throw 'Invalid generated setup credentials.' }
try {
    Invoke-PrivateTool $CertificateExecutable @('initialize', $root, $ServerName) $secrets.certificatePassword
    $passwordFile = Join-Path $root 'initdb-password'
    [IO.File]::WriteAllText($passwordFile, $secrets.adminPassword)
    try {
        Invoke-PrivateTool (Join-Path $bin 'initdb.exe') @('-D', $cluster, '-U', 'postgres', '-A', 'scram-sha-256',
            '--encoding=UTF8', '--locale=C', ('--pwfile=' + $passwordFile))
    } finally {
        if (Test-Path -LiteralPath $passwordFile) { Remove-Item -LiteralPath $passwordFile }
    }
    # The Rust setup authority supplies this isolated loopback port. Never add a LAN listener.
    [IO.File]::AppendAllText((Join-Path $cluster 'postgresql.conf'),
        "`nlisten_addresses = '127.0.0.1'`nport = $DatabasePort`nlog_statement = 'none'`nlog_min_error_statement = 'panic'`n")
    Invoke-PrivateTool (Join-Path $bin 'pg_ctl.exe') @('-D', $cluster, '-l', (Join-Path $root 'postgres.log'), '-w', 'start')
} finally {
    $secrets.adminPassword = $null
    $secrets.certificatePassword = $null
}

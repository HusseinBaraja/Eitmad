#Requires -Version 7.4
[CmdletBinding()]
param(
    [ValidateSet('Start', 'Stop', 'Status')][string]$Action = 'Start',
    [string]$StateDirectory,
    [string]$PostgresBin,
    [ValidateRange(1024, 65535)][int]$DatabasePort = 55439,
    [ValidateRange(1024, 65535)][int]$ServerPort = 58443,
    [switch]$SkipBuild,
    [switch]$VerifySignIn,
    [switch]$DiagnosticActivation,
    [switch]$ServicesOnly
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$workspace = Split-Path $PSScriptRoot -Parent
if ($DiagnosticActivation) {
    if ($ServicesOnly -or $Action -ne 'Start') { throw 'DiagnosticActivation requires Start with the desktop verification environment.' }
    $VerifySignIn = $true
}
if ($ServicesOnly -and $VerifySignIn) { throw 'ServicesOnly cannot verify desktop sign-in.' }
$allowed = [IO.Path]::GetFullPath((Join-Path $workspace 'tests/engine-integration/artifacts'))
if (-not $StateDirectory) { $StateDirectory = Join-Path $allowed 'workflow-e2e' }
$root = [IO.Path]::GetFullPath($StateDirectory)
if (-not $root.StartsWith($allowed + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Test state must be a named child of tests/engine-integration/artifacts. No state was changed.'
}
# Reject junctions and symlinks before reading credentials or starting any service.
$candidate = $root
while ($candidate) {
    if (Test-Path -LiteralPath $candidate) {
        if ((Get-Item -LiteralPath $candidate -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) {
            throw 'Test state cannot use a reparse point.'
        }
    }
    $candidate = Split-Path $candidate -Parent
}
$marker = Join-Path $root 'disposable-eitmad-e2e'
$secretsFile = Join-Path $root 'secrets.json'
$processFile = Join-Path $root 'processes.json'
if (Test-Path -LiteralPath $root) {
    if (-not (Test-Path -LiteralPath $marker) -or
        (Get-Content -LiteralPath $marker -Raw) -cne 'eitmad-workflow-e2e-v1') {
        throw 'Refusing an existing directory without the exact disposable Eitmad test marker. No reset is supported.'
    }
} elseif ($Action -ne 'Start') { throw 'This disposable environment does not exist.' }
if ($ServicesOnly -and $Action -eq 'Start') {
    foreach ($file in 'secrets.json', 'provision.json', 'postgres/PG_VERSION') {
        if (-not (Test-Path -LiteralPath (Join-Path $root $file) -PathType Leaf)) {
            throw 'Services require an existing provisioned installation. Complete the connected workflow setup first.'
        }
    }
    $journal = Get-Content -LiteralPath (Join-Path $root 'provision.json') -Raw | ConvertFrom-Json
    if ($journal.seeded -ne $true) {
        throw 'Services require completed provisioning. No business records were seeded.'
    }
}

function Run-Tool([string]$File, [string[]]$Arguments) {
    & $File @Arguments
    if ($LASTEXITCODE -ne 0) { throw "Tool failed: $([IO.Path]::GetFileName($File)) (exit $LASTEXITCODE)." }
}
function Start-Owned([string]$File, [string[]]$Arguments, [string]$Name, [switch]$Visible) {
    # All generated paths are quoted because Start-Process joins ArgumentList on Windows.
    $quoted = @($Arguments | ForEach-Object {
        if ($_ -match '["\r\n]') { throw 'Invalid process argument.' }
        '"' + $_ + '"'
    })
    $launch = "$Name-$([DateTime]::UtcNow.ToString('yyyyMMddTHHmmssfffZ'))-$([Guid]::NewGuid().ToString('N'))"
    $process = Start-Process -FilePath $File -ArgumentList $quoted -WindowStyle $(if ($Visible) { 'Normal' } else { 'Hidden' }) -PassThru `
        -RedirectStandardOutput (Join-Path $root "$launch.stdout.log") `
        -RedirectStandardError (Join-Path $root "$launch.stderr.log")
    return $process
}
function Find-Owned($Entry) {
    $process = Get-Process -Id $Entry.id -ErrorAction SilentlyContinue
    if ($process -and $process.StartTime.ToUniversalTime().Ticks -eq ([DateTime]$Entry.started).ToUniversalTime().Ticks -and
        [IO.Path]::GetFullPath($process.Path) -eq $Entry.path) { return $process }
    return $null
}
function Record-Process($Process, [string]$Name, [string]$Executable) {
    # A newly started GUI process can return an empty Path from its cached Process object.
    # Record the known launch path and read the creation time from a fresh process handle.
    $observed = Get-Process -Id $Process.Id -ErrorAction Stop
    $script:owned += @{ name = $Name; id = $Process.Id; started = $observed.StartTime.ToUniversalTime().ToString('O'); path = [IO.Path]::GetFullPath($Executable) }
    $script:owned | ConvertTo-Json -AsArray | Set-Content -LiteralPath $processFile
}

$owned = @()
if (Test-Path -LiteralPath $processFile) { $owned = @(Get-Content -LiteralPath $processFile -Raw | ConvertFrom-Json) }
if ($Action -eq 'Status') {
    foreach ($entry in $owned | Where-Object { -not $ServicesOnly -or $_.name -eq 'server' }) {
        Write-Host "$($entry.name): $(if (Find-Owned $entry) { 'running' } else { 'stopped' })"
    }
    if (Test-Path -LiteralPath $secretsFile) {
        $settings = Get-Content -LiteralPath $secretsFile -Raw | ConvertFrom-Json
        & (Join-Path $settings.postgres_bin 'pg_ctl.exe') -D (Join-Path $root 'postgres') status *> $null
        Write-Host "postgres: $(if ($LASTEXITCODE -eq 0) { 'running' } else { 'stopped' })"
    }
    Write-Host "Disposable state: $root"
    return
}
if ($Action -eq 'Stop') {
    $settings = Get-Content -LiteralPath $secretsFile -Raw | ConvertFrom-Json
    # Closing the shell main window hides it in the tray. WM_QUIT ends its dispatcher;
    # process exit closes its kill-on-close engine Job Object.
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class EitmadTestShutdown {
    [DllImport("user32.dll", SetLastError=true)]
    public static extern bool PostThreadMessage(uint thread, uint message, UIntPtr wParam, IntPtr lParam);
}
'@
    foreach ($entry in $owned | Where-Object { $_.name -ne 'server' -and -not $ServicesOnly }) {
        $process = Find-Owned $entry
        if ($process) {
            foreach ($thread in $process.Threads) { [void][EitmadTestShutdown]::PostThreadMessage([uint32]$thread.Id, 0x0012, [UIntPtr]::Zero, [IntPtr]::Zero) }
            if (-not $process.WaitForExit(20000)) { throw 'Client did not stop. Use Exit Eitmad in its tray menu, then retry Stop.' }
        }
    }
    foreach ($entry in $owned | Where-Object name -eq 'server') {
        $process = Find-Owned $entry
        if ($process) { Stop-Process -InputObject $process; $process.WaitForExit() }
    }
    # PostgreSQL fast shutdown commits/rolls back active transactions and preserves the cluster.
    $stop = Start-Owned (Join-Path $settings.postgres_bin 'pg_ctl.exe') @('-D', (Join-Path $root 'postgres'), '-m', 'fast', '-w', 'stop') 'postgres-stop'
    $stop.WaitForExit()
    if ($stop.ExitCode -ne 0) {
        & (Join-Path $settings.postgres_bin 'pg_ctl.exe') -D (Join-Path $root 'postgres') status *> $null
        if ($LASTEXITCODE -eq 0) { throw 'PostgreSQL did not stop.' }
    }
    if ($ServicesOnly) { Write-Host 'Server and PostgreSQL stopped. Accepted work is retained.' }
    else { Write-Host 'Clients, server, and PostgreSQL stopped. Accepted work is retained.' }
    return
}
if (@($owned | Where-Object { Find-Owned $_ }).Count) { throw 'This environment has running processes. Use Stop before Start.' }
if (Test-Path -LiteralPath $secretsFile) {
    $settings = Get-Content -LiteralPath $secretsFile -Raw | ConvertFrom-Json
    $PostgresBin = $settings.postgres_bin
    $DatabasePort = $settings.database_port
    $ServerPort = $settings.server_port
}
if (-not $PostgresBin) { throw 'Supply -PostgresBin with the PostgreSQL 17 binary directory for initial setup.' }
$PostgresBin = [IO.Path]::GetFullPath($PostgresBin)
foreach ($tool in 'initdb', 'pg_ctl', 'psql', 'createdb') {
    if (-not (Test-Path -LiteralPath (Join-Path $PostgresBin "$tool.exe"))) {
        throw "Install PostgreSQL 17 Windows binaries and supply -PostgresBin. Missing: $tool.exe"
    }
}
if ($DatabasePort -eq $ServerPort) { throw 'Database and server ports must differ.' }
if (-not (Test-Path -LiteralPath $root)) {
    New-Item -ItemType Directory -Path $root | Out-Null
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent().Name
    & icacls.exe $root /inheritance:r /grant:r "${identity}:(OI)(CI)F" *> $null
    if ($LASTEXITCODE -ne 0) { throw 'Cannot protect disposable test state.' }
    [IO.File]::WriteAllText($marker, 'eitmad-workflow-e2e-v1')
}
if (-not (Test-Path -LiteralPath $secretsFile)) {
    $admin = [Convert]::ToHexString([Security.Cryptography.RandomNumberGenerator]::GetBytes(32))
    $databasePassword = [Convert]::ToHexString([Security.Cryptography.RandomNumberGenerator]::GetBytes(32))
    $settings = [ordered]@{
        postgres_bin = $PostgresBin; database_port = $DatabasePort; server_port = $ServerPort; admin_password = $admin
        database_password = $databasePassword
        database_url = "postgresql://eitmad_workflow:$databasePassword@127.0.0.1:$DatabasePort/eitmad_workflow_test"
        token_key = [Convert]::ToBase64String([Security.Cryptography.RandomNumberGenerator]::GetBytes(32)).TrimEnd('=').Replace('+', '-').Replace('/', '_')
        endpoint = "https://localhost:$ServerPort/"
        manager_password = [Convert]::ToHexString([Security.Cryptography.RandomNumberGenerator]::GetBytes(16))
        receptionist_password = [Convert]::ToHexString([Security.Cryptography.RandomNumberGenerator]::GetBytes(16))
        manager_seed = @([Security.Cryptography.RandomNumberGenerator]::GetBytes(32))
        receptionist_seed = @([Security.Cryptography.RandomNumberGenerator]::GetBytes(32))
        manager_device = [Guid]::NewGuid().ToString(); receptionist_device = [Guid]::NewGuid().ToString()
    }
    $settings | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $secretsFile
}
$owned = @()
$previous = @{}
$names = @('PGPASSWORD', 'CARGO_INCREMENTAL', 'CARGO_TARGET_DIR', 'EITMAD_E2E_ROOT', 'EITMAD_DIAGNOSTICS') +
    @(Get-ChildItem Env:EITMAD_* | ForEach-Object Name) + @(
    'EITMAD_SERVER_DATABASE_URL', 'EITMAD_SERVER_TOKEN_KEY', 'EITMAD_SERVER_LISTEN',
    'EITMAD_SERVER_TLS_CERTIFICATE', 'EITMAD_SERVER_TLS_PRIVATE_KEY', 'EITMAD_SERVER_UPDATE_MANIFEST_DIRECTORY',
    'EITMAD_SERVER_UPDATE_OPERATOR_TENANT_ID', 'EITMAD_SERVER_UPDATE_KEY_ID', 'EITMAD_SERVER_UPDATE_PUBLIC_KEY')
foreach ($prefix in 'MEDIA', 'PRICING', 'QUOTATION', 'CUSTOMER') {
    $names += @("EITMAD_${prefix}_SERVER", "EITMAD_${prefix}_TRUST_PEM", "EITMAD_${prefix}_CREDENTIAL_ID")
}
$names += @('EITMAD_QUOTATION_ORGANIZATION_ID', 'EITMAD_QUOTATION_BRANCH_ID', 'EITMAD_CUSTOMER_BRANCH_ID', 'EITMAD_E2E_ENGINE')
$names = @($names | Select-Object -Unique)
foreach ($name in $names) { $previous[$name] = [Environment]::GetEnvironmentVariable($name, 'Process'); [Environment]::SetEnvironmentVariable($name, $null, 'Process') }
Push-Location $workspace
$previousPath = $env:PATH
try {
    # Some Windows installs put a system Rust ahead of rustup's proxies.
    # Cargo's subprocesses must use the same required compiler and tools.
    $cargoPath = & rustup which --toolchain 1.85.1 cargo
    if ($LASTEXITCODE -ne 0) { throw 'Install Rust 1.85.1 with rustup.' }
    $env:PATH = (Split-Path $cargoPath -Parent) + [IO.Path]::PathSeparator + $previousPath
    $env:CARGO_TARGET_DIR = Join-Path $workspace 'target/toolchain-1.85.1'
    $env:CARGO_INCREMENTAL = '0'
    if (-not $SkipBuild) {
        $packages = @('-p', 'eitmad-server')
        if (-not $ServicesOnly) { $packages += @('-p', 'eitmad-engine-cli') }
        Run-Tool 'rustup' (@('run', '1.85.1', 'cargo', 'build', '--locked') + $packages)
        Run-Tool 'rustup' @('run', '1.85.1', 'cargo', 'build', '--locked', '-p', 'eitmad-server-connection', '--example', 'workflow_environment')
        if (-not $ServicesOnly) {
            Run-Tool 'dotnet' @('build', 'shells/windows/Eitmad.WindowsShell.csproj', '--configuration', 'Release', '--nologo')
        }
        Run-Tool 'dotnet' @('build', 'tools/dev-certificate/Eitmad.DevCertificate.csproj', '--configuration', 'Release', '--nologo')
        if ($VerifySignIn) {
            Run-Tool 'dotnet' @('build', 'shells/windows/tests/Eitmad.WindowsShell.Tests.csproj', '--configuration', 'Release', '--nologo', '-m:1')
        }
    }
    $server = Join-Path $env:CARGO_TARGET_DIR 'debug/eitmad-server.exe'
    $engine = Join-Path $env:CARGO_TARGET_DIR 'debug/eitmad-engine-cli.exe'
    $provision = Join-Path $env:CARGO_TARGET_DIR 'debug/examples/workflow_environment.exe'
    $shell = Join-Path $workspace 'shells/windows/bin/Release/net8.0-windows/Eitmad.WindowsShell.exe'
    $certificateTool = Join-Path $workspace 'tools/dev-certificate/bin/Release/net8.0/Eitmad.DevCertificate.dll'
    $testAssembly = Join-Path $workspace 'shells/windows/tests/bin/Release/net8.0-windows/Eitmad.WindowsShell.Tests.dll'
    $outputs = @($server, $provision, $certificateTool)
    if (-not $ServicesOnly) { $outputs += @($engine, $shell) }
    if ($VerifySignIn) { $outputs += $testAssembly }
    foreach ($file in $outputs) { if (-not (Test-Path -LiteralPath $file)) { throw 'A build output is missing. Run Start without -SkipBuild.' } }
    $cluster = Join-Path $root 'postgres'
    if (-not (Test-Path -LiteralPath (Join-Path $cluster 'PG_VERSION'))) {
        $passwordFile = Join-Path $root 'initdb-password'
        [IO.File]::WriteAllText($passwordFile, $settings.admin_password)
        try { Run-Tool (Join-Path $PostgresBin 'initdb.exe') @('-D', $cluster, '-U', 'postgres', '-A', 'scram-sha-256', '--encoding=UTF8', '--locale=C', "--pwfile=$passwordFile") }
        finally { Remove-Item -LiteralPath $passwordFile }
    }
    & (Join-Path $PostgresBin 'pg_ctl.exe') -D $cluster status *> $null
    if ($LASTEXITCODE -ne 0) {
        $start = Start-Owned (Join-Path $PostgresBin 'pg_ctl.exe') @('-D', $cluster, '-l', (Join-Path $root ('postgres-' + [Guid]::NewGuid().ToString('N') + '.log')), '-o', "-h 127.0.0.1 -p $DatabasePort -c log_statement=none -c log_min_error_statement=panic", '-w', 'start') 'postgres-start'
        $start.WaitForExit()
        if ($start.ExitCode -ne 0) { throw 'Isolated PostgreSQL startup failed. Check port availability and private logs.' }
    }
    $env:PGPASSWORD = $settings.admin_password
    $connection = @('-h', '127.0.0.1', '-p', "$DatabasePort", '-U', 'postgres', '-d', 'postgres', '-v', 'ON_ERROR_STOP=1')
    $exists = & (Join-Path $PostgresBin 'psql.exe') @connection '-tAc' "SELECT 1 FROM pg_roles WHERE rolname='eitmad_workflow'"
    if ($LASTEXITCODE -ne 0) { throw 'Isolated database authentication failed.' }
    if ($exists -notcontains '1') {
        if ($ServicesOnly) { throw 'The provisioned database role is missing. No role was created.' }
        $roleFile = Join-Path $root 'create-role.sql'
        [IO.File]::WriteAllText($roleFile, "CREATE ROLE eitmad_workflow LOGIN NOSUPERUSER NOBYPASSRLS NOCREATEDB NOCREATEROLE PASSWORD '$($settings.database_password)';")
        try { Run-Tool (Join-Path $PostgresBin 'psql.exe') ($connection + @('-f', $roleFile)) }
        finally { Remove-Item -LiteralPath $roleFile }
    }
    $exists = & (Join-Path $PostgresBin 'psql.exe') @connection '-tAc' "SELECT 1 FROM pg_database WHERE datname='eitmad_workflow_test'"
    if ($LASTEXITCODE -ne 0) { throw 'Cannot inspect disposable database.' }
    if ($exists -notcontains '1') {
        if ($ServicesOnly) { throw 'The provisioned database is missing. No database was created.' }
        Run-Tool (Join-Path $PostgresBin 'createdb.exe') @('-h', '127.0.0.1', '-p', "$DatabasePort", '-U', 'postgres', '-O', 'eitmad_workflow', 'eitmad_workflow_test')
    }
    $env:EITMAD_SERVER_DATABASE_URL = $settings.database_url
    $env:EITMAD_SERVER_TOKEN_KEY = $settings.token_key
    $env:EITMAD_SERVER_LISTEN = "127.0.0.1:$ServerPort"
    $certificates = Join-Path $root 'certificates'
    # Renew only transport certificates; database, identity, work, and token signing key persist.
    Run-Tool 'dotnet' @($certificateTool, $certificates)
    $env:EITMAD_SERVER_TLS_CERTIFICATE = Join-Path $certificates 'server-cert.pem'
    $env:EITMAD_SERVER_TLS_PRIVATE_KEY = Join-Path $certificates 'server-key.pem'
    $env:EITMAD_SERVER_UPDATE_MANIFEST_DIRECTORY = Join-Path $root 'update-manifests'
    New-Item -ItemType Directory -Force -Path $env:EITMAD_SERVER_UPDATE_MANIFEST_DIRECTORY | Out-Null
    $env:EITMAD_SERVER_UPDATE_OPERATOR_TENANT_ID = $settings.manager_device
    $env:EITMAD_SERVER_UPDATE_KEY_ID = 'workflow-e2e'
    # Public Ed25519 test vector only. No update signing private key is installed.
    $env:EITMAD_SERVER_UPDATE_PUBLIC_KEY = '11qYAYKxCrfVS/7TyWQHOg7hcvPapiMlrwIaaPcHURo='
    Run-Tool $server @('migrate')
    $process = Start-Owned $server @('serve') 'server'
    Record-Process $process 'server' $server
    $readyUntil = [DateTime]::UtcNow.AddSeconds(20)
    $handler = [Net.Http.SocketsHttpHandler]::new()
    $chain = [Security.Cryptography.X509Certificates.X509ChainPolicy]::new()
    $chain.TrustMode = [Security.Cryptography.X509Certificates.X509ChainTrustMode]::CustomRootTrust
    # This generated private CA has no revocation service. Validate its chain and hostname,
    # with no certificate-validation callback or acceptance of an unrelated CA.
    $chain.RevocationMode = [Security.Cryptography.X509Certificates.X509RevocationMode]::NoCheck
    $ca = [Security.Cryptography.X509Certificates.X509Certificate2]::CreateFromPem([IO.File]::ReadAllText((Join-Path $certificates 'trusted-ca.pem')))
    [void]$chain.CustomTrustStore.Add($ca)
    $handler.SslOptions.CertificateChainPolicy = $chain
    $http = [Net.Http.HttpClient]::new($handler)
    $http.Timeout = [TimeSpan]::FromSeconds(2)
    try {
        do {
            if ($process.HasExited) { throw 'The authenticated server stopped during startup.' }
            try {
                $response = $http.GetAsync("$($settings.endpoint)readyz").GetAwaiter().GetResult()
                $ready = $response.StatusCode -eq [Net.HttpStatusCode]::NoContent
                $response.Dispose()
            } catch { $ready = $false }
            if (-not $ready) { Start-Sleep -Milliseconds 200 }
        } until ($ready -or [DateTime]::UtcNow -gt $readyUntil)
    } finally { $http.Dispose(); $ca.Dispose() }
    if (-not $ready) { throw 'Server readiness deadline elapsed.' }
    $env:EITMAD_E2E_ROOT = $root
    Run-Tool $provision @()
    if ($ServicesOnly) {
        Write-Host 'SERVICES READY: PostgreSQL and the TLS server are running. Connection references are refreshed; business records are retained.'
        Write-Host "Open the app separately: .\run.ps1 -RuntimeDirectory `"$(Join-Path $root 'receptionist')`""
        return
    }
    # The shell and its engine must not inherit server credentials.
    foreach ($name in 'PGPASSWORD', 'EITMAD_SERVER_DATABASE_URL', 'EITMAD_SERVER_TOKEN_KEY') {
        [Environment]::SetEnvironmentVariable($name, $null, 'Process')
    }
    if ($VerifySignIn) {
        $env:EITMAD_E2E_ENGINE = $engine
        $testName = if ($DiagnosticActivation) { 'DiagnosticAccountSwitchAndConfirmation' } else { 'OneDesktopSwitchesAccountsAndRetainsServerConfirmedWork' }
        $filter = "FullyQualifiedName=Eitmad.WindowsShell.Tests.Rendered.WorkflowEnvironmentTests.$testName"
        if ($DiagnosticActivation) { $env:EITMAD_DIAGNOSTICS = '1' }
        $results = Join-Path $root ('launch-check-' + [Guid]::NewGuid().ToString('N'))
        if ($DiagnosticActivation) {
            $testProcess = Start-Owned 'dotnet' @('vstest', $testAssembly, "--TestCaseFilter:$filter", "--ResultsDirectory:$results", '--Logger:trx;LogFileName=workflow.trx') 'activation-diagnostic'
            $testProcess.WaitForExit()
            if ($testProcess.ExitCode -ne 0) { throw 'Activation diagnostic failed. Retain the distinct activation-diagnostic stdout/stderr and TRX evidence.' }
        } else {
            Run-Tool 'dotnet' @('vstest', $testAssembly, "--TestCaseFilter:$filter", "--ResultsDirectory:$results", '--Logger:trx;LogFileName=workflow.trx')
        }
        [xml]$result = Get-Content -LiteralPath (Join-Path $results 'workflow.trx') -Raw
        $counts = $result.TestRun.ResultSummary.Counters
        if ([int]$counts.total -lt 1 -or [int]$counts.passed -ne [int]$counts.total) {
            throw 'Workflow verification did not pass every selected test.'
        }
    }
    if ($DiagnosticActivation) {
        Write-Host "Activation diagnostic passed. Retain stdout/stderr logs and TRX in: $root"
        return
    }
    $directory = Join-Path $root 'receptionist'
    $client = Start-Owned $shell @('--engine', $engine, '--runtime-directory', $directory) 'desktop' -Visible
    Record-Process $client 'desktop' $shell
    Write-Host 'One real WPF client started. Sign-in names: manager and receptionist. Use Alt+K to switch accounts.'
    Write-Host "Read the generated passwords locally from the protected file: $secretsFile"
    Write-Host "Disposable state is retained at: $root"
} finally {
    $env:PATH = $previousPath
    Pop-Location
    foreach ($name in $names) { [Environment]::SetEnvironmentVariable($name, $previous[$name], 'Process') }
}

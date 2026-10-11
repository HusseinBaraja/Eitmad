# Uses only new synthetic databases. Requires tools/dev-database.ps1 to have installed PostgreSQL.
$ErrorActionPreference = 'Stop'
$workspace = Split-Path $PSScriptRoot -Parent
$postgresRoot = Join-Path $workspace 'target/direct-order-test'
$bin = Join-Path $postgresRoot 'pgsql/bin'
$credentialFile = Join-Path $postgresRoot 'credentials.json'
if (-not (Test-Path -LiteralPath $credentialFile) -or -not (Test-Path -LiteralPath (Join-Path $bin 'pg_dump.exe'))) {
    throw 'Run tools/dev-database.ps1 first to install isolated loopback PostgreSQL tools.'
}
$credentials = Get-Content -LiteralPath $credentialFile -Raw | ConvertFrom-Json
$runId = [Guid]::NewGuid().ToString('N')
$root = Join-Path $workspace "tests/engine-integration/artifacts/recovery-$runId"
New-Item -ItemType Directory -Path $root | Out-Null
$identity = [Security.Principal.WindowsIdentity]::GetCurrent().Name
& icacls.exe $root /inheritance:r /grant:r "${identity}:(OI)(CI)F" *> $null
if ($LASTEXITCODE -ne 0) { throw 'Cannot protect recovery artifacts.' }
$cluster = Join-Path $postgresRoot 'data'
& (Join-Path $bin 'pg_isready.exe') --host 127.0.0.1 --port $credentials.Port *> $null
if ($LASTEXITCODE -ne 0) {
    if (-not (Test-Path -LiteralPath (Join-Path $cluster 'PG_VERSION'))) { throw 'The isolated test cluster is not initialized.' }
    $startupArguments = '-D "{0}" -l "{1}" -o "-h 127.0.0.1 -p {2}" -w start' -f $cluster, (Join-Path $postgresRoot 'postgresql.log'), $credentials.Port
    $startup = Start-Process -FilePath (Join-Path $bin 'pg_ctl.exe') -ArgumentList $startupArguments -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $root 'cluster-start.stdout') -RedirectStandardError (Join-Path $root 'cluster-start.stderr')
    if (-not $startup.WaitForExit(30000) -or $startup.ExitCode -ne 0) { throw 'Cannot start the existing isolated test cluster.' }
}
$names = @('PGPASSWORD', 'CARGO_INCREMENTAL', 'EITMAD_DIRECT_TEST_DATABASE_URL', 'EITMAD_DIRECT_TEST_CERTIFICATE',
    'EITMAD_DIRECT_TEST_PRIVATE_KEY', 'EITMAD_DIRECT_TEST_TRUSTED_CERTIFICATE',
    'EITMAD_RECOVERY_ARTIFACT_DIRECTORY', 'EITMAD_RECOVERY_PG_BIN', 'EITMAD_RECOVERY_ADMIN_PASSWORD')
$previous = @{}
foreach ($name in $names) { $previous[$name] = [Environment]::GetEnvironmentVariable($name, 'Process') }
try {
    $env:PGPASSWORD = $credentials.Admin
    $database = 'eitmad_recovery_' + $runId
    & (Join-Path $bin 'createdb.exe') --host 127.0.0.1 --port $credentials.Port --username postgres --owner eitmad_test $database *> $null
    if ($LASTEXITCODE -ne 0) { throw 'Cannot create a new isolated recovery database.' }
    $env:EITMAD_DIRECT_TEST_DATABASE_URL = "postgresql://eitmad_test:$($credentials.Test)@127.0.0.1:$($credentials.Port)/$database"
    $certificates = Join-Path $root 'certificates'
    & dotnet run --project (Join-Path $workspace 'tools/dev-certificate/Eitmad.DevCertificate.csproj') --configuration Release -- $certificates
    if ($LASTEXITCODE -ne 0) { throw 'Cannot create isolated synthetic TLS certificates.' }
    $env:EITMAD_DIRECT_TEST_CERTIFICATE = Join-Path $certificates 'server-cert.pem'
    $env:EITMAD_DIRECT_TEST_PRIVATE_KEY = Join-Path $certificates 'server-key.pem'
    $env:EITMAD_DIRECT_TEST_TRUSTED_CERTIFICATE = Join-Path $certificates 'trusted-ca.pem'
    $env:EITMAD_RECOVERY_ARTIFACT_DIRECTORY = $root
    $env:EITMAD_RECOVERY_PG_BIN = $bin
    $env:EITMAD_RECOVERY_ADMIN_PASSWORD = $credentials.Admin
    $env:CARGO_INCREMENTAL = '0'
    & rustup run 1.85.1 cargo test --locked -p eitmad-server-connection --test direct_route manual_two_client_recovery -- --ignored --nocapture
    if ($LASTEXITCODE -ne 0) { throw 'Manual recovery drill failed. Private artifacts and both databases are preserved.' }
    Get-Content -LiteralPath (Join-Path $root 'result.json')
} finally {
    foreach ($name in $names) { [Environment]::SetEnvironmentVariable($name, $previous[$name], 'Process') }
}

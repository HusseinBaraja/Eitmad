param(
    [ValidateSet('Setup', 'NewDatabase', 'Stop')][string]$Action = 'Setup',
    [int]$Port = 55432
)
$ErrorActionPreference = 'Stop'
$workspace = Split-Path $PSScriptRoot -Parent
$testRoot = Join-Path $workspace 'target/direct-order-test'
$databaseDirectory = Join-Path $testRoot 'data'
$bin = Join-Path $testRoot 'pgsql/bin'
$credentialFile = Join-Path $testRoot 'credentials.json'
New-Item -ItemType Directory -Force -Path $testRoot | Out-Null

function Invoke-DatabaseTool([string]$Name, [string[]]$Arguments) {
    & (Join-Path $bin "$Name.exe") @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Name failed with exit code $LASTEXITCODE" }
}
if ($Action -eq 'Stop') {
    Invoke-DatabaseTool 'pg_ctl' @('-D', $databaseDirectory, '-m', 'fast', '-w', 'stop')
    return
}
if (-not (Test-Path -LiteralPath (Join-Path $bin 'initdb.exe'))) {
    $archive = Join-Path $testRoot 'postgresql.zip'
    if (-not (Test-Path -LiteralPath $archive)) {
        Invoke-WebRequest 'https://get.enterprisedb.com/postgresql/postgresql-17.11-3-windows-x64-binaries.zip' -OutFile $archive
    }
    $expectedHash = '4B8DB0930C38F6EF845DB919551DEDDA3B6B845AEB0927B3D79A6E8E9E4537CF'
    if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne $expectedHash) { throw 'PostgreSQL archive checksum does not match the pinned development binary' }
    Expand-Archive -LiteralPath $archive -DestinationPath $testRoot
    (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash | Set-Content (Join-Path $testRoot 'postgresql.sha256')
}
if (-not (Test-Path -LiteralPath $credentialFile)) {
    $credentials = @{ Admin = [Convert]::ToHexString([Security.Cryptography.RandomNumberGenerator]::GetBytes(32)); Test = [Convert]::ToHexString([Security.Cryptography.RandomNumberGenerator]::GetBytes(32)); Port = $Port }
    $credentials | ConvertTo-Json | Set-Content -LiteralPath $credentialFile
    # Keep credentials and the database cluster private to this Windows account.
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent().Name
    & icacls.exe $testRoot /inheritance:r /grant:r "${identity}:(OI)(CI)F" | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Cannot restrict local test directory permissions' }
}
$credentials = Get-Content -LiteralPath $credentialFile -Raw | ConvertFrom-Json
$Port = $credentials.Port
if (-not (Test-Path -LiteralPath (Join-Path $databaseDirectory 'PG_VERSION'))) {
    $passwordFile = Join-Path $testRoot 'initdb-password'
    $credentials.Admin | Set-Content -LiteralPath $passwordFile -NoNewline
    try { Invoke-DatabaseTool 'initdb' @('-D', $databaseDirectory, '-U', 'postgres', '-A', 'scram-sha-256', '--encoding=UTF8', '--locale=C', "--pwfile=$passwordFile") }
    finally { Remove-Item -LiteralPath $passwordFile }
}
& (Join-Path $bin 'pg_ctl.exe') -D $databaseDirectory status *> $null
if ($LASTEXITCODE -ne 0) {
    Invoke-DatabaseTool 'pg_ctl' @('-D', $databaseDirectory, '-l', (Join-Path $testRoot 'postgresql.log'), '-o', "-h 127.0.0.1 -p $Port", '-w', 'start')
}
$previousPassword = $env:PGPASSWORD
try {
    $env:PGPASSWORD = $credentials.Admin
    $connection = @('-h', '127.0.0.1', '-p', "$Port", '-U', 'postgres', '-d', 'postgres', '-v', 'ON_ERROR_STOP=1')
    $roleExists = & (Join-Path $bin 'psql.exe') @connection '-tAc' "SELECT 1 FROM pg_roles WHERE rolname='eitmad_test'"
    if ($LASTEXITCODE -ne 0) { throw 'Cannot inspect disposable test role' }
    if ($roleExists -notcontains '1') {
        $roleFile = Join-Path $testRoot 'create-test-role.sql'
        "CREATE ROLE eitmad_test LOGIN NOSUPERUSER NOBYPASSRLS NOCREATEDB NOCREATEROLE PASSWORD '$($credentials.Test)';" | Set-Content -LiteralPath $roleFile
        try { Invoke-DatabaseTool 'psql' ($connection + @('-f', $roleFile)) }
        finally { Remove-Item -LiteralPath $roleFile }
    }
    $databaseName = 'eitmad_test_' + [Guid]::NewGuid().ToString('N')
    Invoke-DatabaseTool 'createdb' @('-h', '127.0.0.1', '-p', "$Port", '-U', 'postgres', '-O', 'eitmad_test', $databaseName)
    $env:EITMAD_DIRECT_TEST_DATABASE_URL = "postgresql://eitmad_test:$($credentials.Test)@127.0.0.1:$Port/$databaseName"
} finally { $env:PGPASSWORD = $previousPassword }
$certDirectory = Join-Path $workspace 'target/direct-route-cert'
& dotnet run --project (Join-Path $workspace 'tools/dev-certificate/Eitmad.DevCertificate.csproj') --configuration Release -- $certDirectory
if ($LASTEXITCODE -ne 0) { throw 'Cannot create disposable TLS certificates' }
$env:EITMAD_DIRECT_TEST_CERTIFICATE = Join-Path $certDirectory 'server-cert.pem'
$env:EITMAD_DIRECT_TEST_PRIVATE_KEY = Join-Path $certDirectory 'server-key.pem'
$env:EITMAD_DIRECT_TEST_TRUSTED_CERTIFICATE = Join-Path $certDirectory 'trusted-ca.pem'
$env:EITMAD_DIRECT_TEST_WRONG_CERTIFICATE = Join-Path $certDirectory 'wrong-ca.pem'
Write-Host "Disposable PostgreSQL database created on loopback port $Port. Credentials stay in the ignored test directory."
Write-Host 'Dot-source this script to retain EITMAD_DIRECT_TEST_* settings in your current PowerShell session.'

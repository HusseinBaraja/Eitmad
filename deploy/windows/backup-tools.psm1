Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Invoke-BackupTool([string]$File, [string[]]$Arguments) {
    # Never publish native diagnostics: tools can include credentials or private paths.
    & $File @Arguments *> $null
    if ($LASTEXITCODE -ne 0) { throw 'Backup tool failed. Inspect the protected installation.' }
}

function Backup-Postgres([string]$Bin, [int]$Port, [pscredential]$Credential, [string]$Target) {
    $previous = @{}
    foreach ($key in @(Get-ChildItem Env: | Where-Object Name -like 'PG*')) {
        $previous[$key.Name] = $key.Value
        Remove-Item ('Env:' + $key.Name)
    }
    try {
        $env:PGPASSWORD = $Credential.GetNetworkCredential().Password
        Invoke-BackupTool (Join-Path $Bin 'pg_basebackup.exe') @('--host=127.0.0.1',
            ('--port=' + $Port), ('--username=' + $Credential.UserName), '--no-password', '--format=plain',
            '--wal-method=stream', '--checkpoint=fast', '--manifest-checksums=SHA256', ('--pgdata=' + $Target))
        Invoke-BackupTool (Join-Path $Bin 'pg_verifybackup.exe') @('--exit-on-error', $Target)
    } finally {
        Remove-Item Env:PGPASSWORD -ErrorAction SilentlyContinue
        foreach ($key in $previous.Keys) { Set-Item ('Env:' + $key) $previous[$key] }
    }
}

Export-ModuleMember -Function Invoke-BackupTool, Backup-Postgres

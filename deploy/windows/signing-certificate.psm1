#Requires -Version 7.4
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Assert-EitmadSelfSignedCertificate {
    param([Parameter(Mandatory)][Security.Cryptography.X509Certificates.X509Certificate2]$Certificate,
        [string]$Publisher)
    $eku = @($Certificate.Extensions | Where-Object { $_.Oid.Value -eq '2.5.29.37' })
    $constraints = @($Certificate.Extensions | Where-Object { $_.Oid.Value -eq '2.5.29.19' })
    if ($Certificate.Subject -cne $Certificate.Issuer -or
        ($Publisher -and $Certificate.Subject -cne $Publisher) -or
        $Certificate.NotBefore -gt (Get-Date) -or $Certificate.NotAfter -le (Get-Date) -or
        $eku.Count -ne 1 -or $eku[0].EnhancedKeyUsages.Count -ne 1 -or
        $eku[0].EnhancedKeyUsages[0].Value -cne '1.3.6.1.5.5.7.3.3' -or
        $constraints.Count -ne 1 -or $constraints[0].CertificateAuthority) {
        throw 'Require a valid self-signed code-signing-only leaf certificate with the exact publisher.'
    }
}

function Get-EitmadSigningCertificate {
    $thumbprint = $env:EITMAD_SIGNING_CERTIFICATE_THUMBPRINT
    if ($thumbprint -notmatch '^[a-fA-F0-9]{40}$') {
        throw 'Set EITMAD_SIGNING_CERTIFICATE_THUMBPRINT to the self-signed certificate in CurrentUser\My.'
    }
    $certificate = Get-Item -LiteralPath ('Cert:\CurrentUser\My\' + $thumbprint)
    Assert-EitmadSelfSignedCertificate $certificate
    if (-not $certificate.HasPrivateKey) { throw 'The build certificate must retain its private signing key.' }
    return $certificate
}

Export-ModuleMember -Function Assert-EitmadSelfSignedCertificate, Get-EitmadSigningCertificate

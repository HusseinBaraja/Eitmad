using System.Diagnostics;
using System.Runtime.Versioning;
using System.Security.Cryptography;
using System.Security.Cryptography.X509Certificates;

namespace Eitmad.Platform.Windows.Installation;

/// <summary>Manage explicit Windows trust for the installer's public code-signing leaf.</summary>
[SupportedOSPlatform("windows")]
public sealed class PublisherCertificateTrust : IDisposable
{
    private readonly X509Certificate2 certificate;
    public string Publisher => certificate.Subject;
    public string Fingerprint => certificate.GetCertHashString(HashAlgorithmName.SHA256);

    public PublisherCertificateTrust(byte[] bytes, string expectedSha256)
    {
        if (bytes.Length is 0 or > 16384 || expectedSha256.Length != 64 ||
            !Convert.ToHexString(SHA256.HashData(bytes)).Equals(expectedSha256, StringComparison.OrdinalIgnoreCase))
            throw new IOException("Publisher certificate digest differs.");
        if (X509Certificate2.GetCertContentType(bytes) != X509ContentType.Cert)
            throw new IOException("Publisher resource must contain only a public certificate.");
        certificate = new X509Certificate2(bytes);
        try
        {
            var usages = certificate.Extensions.OfType<X509EnhancedKeyUsageExtension>().ToList();
            var constraints = certificate.Extensions.OfType<X509BasicConstraintsExtension>().ToList();
            if (certificate.HasPrivateKey || certificate.Subject != certificate.Issuer ||
                !certificate.SubjectName.RawData.SequenceEqual(certificate.IssuerName.RawData) ||
                certificate.NotBefore > DateTime.Now || certificate.NotAfter <= DateTime.Now ||
                usages.Count != 1 || usages[0].EnhancedKeyUsages.Count != 1 ||
                usages[0].EnhancedKeyUsages[0].Value != "1.3.6.1.5.5.7.3.3" ||
                constraints.Count != 1 || constraints[0].CertificateAuthority)
                throw new IOException("Publisher must be a valid self-signed code-signing-only leaf.");
        }
        catch { certificate.Dispose(); throw; }
    }

    public void VerifyInstallerSigner(string executable)
    {
        using var signed = X509Certificate.CreateFromSignedFile(executable);
        using var signer = new X509Certificate2(signed);
        if (!CryptographicOperations.FixedTimeEquals(signer.RawData, certificate.RawData))
            throw new IOException("Installer signer differs from its embedded publisher certificate.");
    }

    public bool IsTrustedForMachine()
        => IsInMachineStore(StoreName.Root) && IsInMachineStore(StoreName.TrustedPublisher);

    private bool IsInMachineStore(StoreName name)
    {
        using var store = new X509Store(name, StoreLocation.LocalMachine);
        store.Open(OpenFlags.ReadOnly);
        return store.Certificates.Cast<X509Certificate2>().Any(entry => entry.RawData.SequenceEqual(certificate.RawData));
    }

    public async Task TrustForMachineAsync()
    {
        if (IsTrustedForMachine()) return;
        var root = Path.Combine(Path.GetTempPath(), "EitmadPublisherTrust-" + Guid.NewGuid().ToString("N"));
        EmbeddedInstallerPayload.CreatePrivateDirectory(root);
        var path = Path.Combine(root, "publisher.cer");
        try
        {
            using (var output = new FileStream(path, FileMode.CreateNew, FileAccess.Write, FileShare.None))
                await output.WriteAsync(certificate.RawData);
            using (var lease = OfflinePayloadLease.RetainForLaunch(root))
            {
                // Root validates the signature chain. TrustedPublisher permits
                // AllSigned scripts without an interactive publisher prompt,
                // including under another UAC account and the SYSTEM task.
                foreach (var name in new[] { StoreName.Root, StoreName.TrustedPublisher })
                {
                    if (IsInMachineStore(name)) continue;
                    var start = new ProcessStartInfo(Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.System), "certutil.exe")) {
                        UseShellExecute = true, Verb = "runas", WindowStyle = ProcessWindowStyle.Hidden,
                    };
                    foreach (var argument in new[] { "-addstore", name.ToString(), path }) start.ArgumentList.Add(argument);
                    using var child = Process.Start(start) ?? throw new IOException("Windows certificate trust unavailable.");
                    await child.WaitForExitAsync();
                    if (child.ExitCode != 0 || !IsInMachineStore(name)) throw new IOException("Publisher certificate was not trusted.");
                }
            }
        }
        finally
        {
            // Delete only this invocation's public certificate and empty private directory.
            File.Delete(path); Directory.Delete(root);
        }
    }

    public void Dispose() => certificate.Dispose();
}

using System.IO;
using System.Security.Cryptography;
using System.Security.Cryptography.X509Certificates;
using Eitmad.Platform.Windows.Installation;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace Eitmad.WindowsShell.Tests;

[TestClass]
public sealed class PublisherCertificateTrustTests
{
    private static byte[] Certificate(bool authority = false, bool codeSigning = true, bool expired = false, bool privateKey = false)
    {
        using var rsa = RSA.Create(2048);
        var request = new CertificateRequest("CN=Eitmad synthetic publisher", rsa, HashAlgorithmName.SHA256, RSASignaturePadding.Pkcs1);
        var usages = new OidCollection { new(codeSigning ? "1.3.6.1.5.5.7.3.3" : "1.3.6.1.5.5.7.3.1") };
        request.CertificateExtensions.Add(new X509EnhancedKeyUsageExtension(usages, true));
        request.CertificateExtensions.Add(new X509BasicConstraintsExtension(authority, false, 0, true));
        using var certificate = request.CreateSelfSigned(DateTimeOffset.UtcNow.AddDays(-2),
            DateTimeOffset.UtcNow.AddDays(expired ? -1 : 1));
        return certificate.Export(privateKey ? X509ContentType.Pfx : X509ContentType.Cert);
    }

    [TestMethod]
    public void Public_code_signing_leaf_is_pinned_to_its_exact_digest()
    {
        var bytes = Certificate(); var hash = Convert.ToHexString(SHA256.HashData(bytes));
        using var trust = new PublisherCertificateTrust(bytes, hash);
        Assert.AreEqual(hash, trust.Fingerprint);
        Assert.ThrowsExactly<IOException>(() => new PublisherCertificateTrust(Certificate(), hash));
    }

    [TestMethod]
    [DataRow(true, false, false, false)] [DataRow(false, true, false, false)]
    [DataRow(false, false, true, false)] [DataRow(false, false, false, true)]
    public void Authority_other_purpose_expired_or_private_certificates_are_refused(bool authority, bool wrongPurpose, bool expired, bool privateKey)
    {
        var bytes = Certificate(authority, !wrongPurpose, expired, privateKey);
        Assert.ThrowsExactly<IOException>(() => new PublisherCertificateTrust(bytes, Convert.ToHexString(SHA256.HashData(bytes))));
    }

}

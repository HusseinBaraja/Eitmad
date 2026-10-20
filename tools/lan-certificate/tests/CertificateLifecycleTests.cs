using System.Security.AccessControl;
using System.Security.Cryptography;
using System.Security.Cryptography.X509Certificates;
using System.Security.Principal;
using System.Text.Json;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace Eitmad.LanCertificate.Tests;

[TestClass]
public sealed class CertificateLifecycleTests
{
    private const string Password = "synthetic LAN test password";

    [TestMethod]
    public void RenewalRetainsTrustAndFailedRenewalRetainsActivePair()
    {
        var evidence = Environment.GetEnvironmentVariable("EITMAD_LAN_TEST_ROOT")
            ?? Path.Combine(Path.GetTempPath(), "eitmad-lan-" + Guid.NewGuid().ToString("N"));
        // The Rust TLS test consumes these synthetic bundles. Never use an existing installation.
        Directory.CreateDirectory(evidence);
        var directory = Path.Combine(evidence, "server");
        Certificates.Initialize(directory, "workshop.test", Password);
        Certificates.RequirePrivateDirectory(directory);
        var trust = File.ReadAllBytes(Path.Combine(directory, "trusted-ca.pem"));
        var activePath = Path.Combine(directory, "active-tls.json");
        var first = File.ReadAllText(activePath);
        File.WriteAllText(Path.Combine(directory, "previous-tls.json"), first);
        Assert.ThrowsExactly<CryptographicException>(() => Certificates.Renew(directory, "wrong test password"));
        Assert.AreEqual(first, File.ReadAllText(activePath));
        Assert.ThrowsExactly<IOException>(() => Certificates.Initialize(directory, "other.test", Password));
        Certificates.Renew(directory, Password);
        CollectionAssert.AreEqual(trust, File.ReadAllBytes(Path.Combine(directory, "trusted-ca.pem")));
        Assert.AreNotEqual(first, File.ReadAllText(activePath));
        using var ca = X509Certificate2.CreateFromPem(File.ReadAllText(Path.Combine(directory, "trusted-ca.pem")));
        foreach (var manifest in new[] { first, File.ReadAllText(activePath) })
        {
            var active = JsonSerializer.Deserialize<Certificates.ActiveTls>(manifest)!;
            using var pair = X509Certificate2.CreateFromPemFile(Path.Combine(directory, active.Certificate), Path.Combine(directory, active.PrivateKey));
            using var chain = new X509Chain();
            chain.ChainPolicy.TrustMode = X509ChainTrustMode.CustomRootTrust;
            chain.ChainPolicy.CustomTrustStore.Add(ca);
            chain.ChainPolicy.RevocationMode = X509RevocationMode.NoCheck;
            chain.ChainPolicy.ApplicationPolicy.Add(new Oid("1.3.6.1.5.5.7.3.1"));
            Assert.IsTrue(chain.Build(pair));
            Assert.IsTrue(pair.HasPrivateKey);
        }
        Certificates.Initialize(Path.Combine(evidence, "wrong"), "workshop.test", Password);
        Certificates.Initialize(Path.Combine(evidence, "ip"), "192.168.50.10", Password);
        TestContext.WriteLine($"Synthetic TLS evidence: {evidence}");
    }

    [TestMethod]
    public void BroadPrivateKeyPermissionsPreventRenewal()
    {
        var directory = Path.Combine(Path.GetTempPath(), "eitmad-lan-acl-" + Guid.NewGuid().ToString("N"));
        Certificates.Initialize(directory, "192.168.50.10", Password);
        var activePath = Path.Combine(directory, "active-tls.json");
        var manifest = File.ReadAllText(activePath);
        var active = JsonSerializer.Deserialize<Certificates.ActiveTls>(manifest)!;
        var file = new FileInfo(Path.Combine(directory, active.PrivateKey));
        var security = file.GetAccessControl();
        var broad = new FileSystemAccessRule(new SecurityIdentifier(WellKnownSidType.BuiltinUsersSid, null),
            FileSystemRights.Read, AccessControlType.Allow);
        security.AddAccessRule(broad);
        file.SetAccessControl(security);
        try
        {
            Assert.ThrowsExactly<UnauthorizedAccessException>(() => Certificates.Renew(directory, Password));
            Assert.AreEqual(manifest, File.ReadAllText(activePath));
        }
        finally
        {
            security.RemoveAccessRuleSpecific(broad);
            file.SetAccessControl(security);
        }
    }

    public TestContext TestContext { get; set; } = null!;
}

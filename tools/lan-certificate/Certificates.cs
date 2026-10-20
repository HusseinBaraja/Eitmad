using System.Net;
using System.Formats.Asn1;
using System.Security.AccessControl;
using System.Security.Cryptography;
using System.Security.Cryptography.X509Certificates;
using System.Security.Principal;
using System.Text.Json;

namespace Eitmad.LanCertificate;

public static class Certificates
{
    public sealed record Identity(int Version, string ServerName);
    public sealed record ActiveTls(int Version, string Certificate, string PrivateKey);

    public static void Initialize(string directory, string serverName, string password)
    {
        ValidatePassword(password);
        ValidateName(serverName);
        directory = Path.GetFullPath(directory);
        if (Directory.Exists(directory) || File.Exists(directory))
            throw new IOException("Initialization requires a new directory.");
        RejectReparsePoints(directory);
        CreatePrivateDirectory(directory);
        using var owner = Lock(directory);
        var now = DateTimeOffset.UtcNow;
        using var key = ECDsa.Create(ECCurve.NamedCurves.nistP256);
        var request = new CertificateRequest("CN=Eitmad LAN CA", key, HashAlgorithmName.SHA256);
        request.CertificateExtensions.Add(new X509BasicConstraintsExtension(true, true, 0, true));
        request.CertificateExtensions.Add(new X509KeyUsageExtension(X509KeyUsageFlags.KeyCertSign, true));
        request.CertificateExtensions.Add(new X509SubjectKeyIdentifierExtension(request.PublicKey, false));
        using var ca = request.CreateSelfSigned(now.AddMinutes(-5), now.AddYears(5));
        File.WriteAllText(Path.Combine(directory, "trusted-ca.pem"), ca.ExportCertificatePem());
        File.WriteAllText(Path.Combine(directory, "ca-key.encrypted.pem"), key.ExportEncryptedPkcs8PrivateKeyPem(
            password, new PbeParameters(PbeEncryptionAlgorithm.Aes256Cbc, HashAlgorithmName.SHA256, 210_000)));
        File.WriteAllText(Path.Combine(directory, "identity.json"), JsonSerializer.Serialize(new Identity(1, serverName)));
        Issue(directory, serverName, ca, now);
    }

    public static void Renew(string directory, string password)
    {
        ValidatePassword(password);
        directory = Path.GetFullPath(directory);
        RequirePrivateDirectory(directory);
        using var owner = Lock(directory);
        var identity = JsonSerializer.Deserialize<Identity>(File.ReadAllText(Path.Combine(directory, "identity.json")))
            ?? throw new IOException("Missing identity.");
        if (identity.Version != 1) throw new IOException("Unknown version.");
        ValidateName(identity.ServerName);
        using var key = ECDsa.Create();
        key.ImportFromEncryptedPem(File.ReadAllText(Path.Combine(directory, "ca-key.encrypted.pem")), password);
        using var certificate = X509Certificate2.CreateFromPem(File.ReadAllText(Path.Combine(directory, "trusted-ca.pem")));
        using var ca = certificate.CopyWithPrivateKey(key);
        Issue(directory, identity.ServerName, ca, DateTimeOffset.UtcNow);
    }

    public static string Fingerprint(string directory)
    {
        using var ca = X509Certificate2.CreateFromPem(File.ReadAllText(Path.Combine(directory, "trusted-ca.pem")));
        return ca.GetCertHashString(HashAlgorithmName.SHA256);
    }

    private static void Issue(string directory, string serverName, X509Certificate2 ca, DateTimeOffset now)
    {
        // A new immutable pair is written before the single active pointer changes.
        // Failed renewal cannot leave the running server with a mismatched cert/key pair.
        if (ca.NotBefore.ToUniversalTime() > now.UtcDateTime || ca.NotAfter.ToUniversalTime() < now.AddDays(91).UtcDateTime)
            throw new InvalidOperationException("CA renewal requires an explicit trust transition.");
        var bundle = Guid.NewGuid().ToString("N");
        var target = Path.Combine(directory, "tls", bundle);
        CreatePrivateDirectory(target);
        using var key = ECDsa.Create(ECCurve.NamedCurves.nistP256);
        var request = new CertificateRequest("CN=Eitmad LAN server", key, HashAlgorithmName.SHA256);
        request.CertificateExtensions.Add(new X509BasicConstraintsExtension(false, false, 0, true));
        request.CertificateExtensions.Add(new X509KeyUsageExtension(X509KeyUsageFlags.DigitalSignature, true));
        request.CertificateExtensions.Add(new X509EnhancedKeyUsageExtension(new OidCollection { new("1.3.6.1.5.5.7.3.1") }, true));
        request.CertificateExtensions.Add(new X509SubjectKeyIdentifierExtension(request.PublicKey, false));
        // RFC 5280 requires issuer key identification on certificates issued by this CA.
        // Encode the implicit context tag explicitly because .NET 8 has no typed AKI extension.
        var issuerKey = ca.Extensions.OfType<X509SubjectKeyIdentifierExtension>().Single().SubjectKeyIdentifier
            ?? throw new CryptographicException("The retained authority has no subject key identifier.");
        var authorityKey = new AsnWriter(AsnEncodingRules.DER);
        authorityKey.PushSequence();
        authorityKey.WriteOctetString(Convert.FromHexString(issuerKey), new Asn1Tag(TagClass.ContextSpecific, 0));
        authorityKey.PopSequence();
        request.CertificateExtensions.Add(new X509Extension("2.5.29.35", authorityKey.Encode(), false));
        var names = new SubjectAlternativeNameBuilder();
        if (IPAddress.TryParse(serverName, out var address)) names.AddIpAddress(address);
        else names.AddDnsName(serverName);
        request.CertificateExtensions.Add(names.Build());
        using var leaf = request.Create(ca, now.AddMinutes(-5), now.AddDays(90), RandomNumberGenerator.GetBytes(16));
        File.WriteAllText(Path.Combine(target, "server-cert.pem"), leaf.ExportCertificatePem());
        File.WriteAllText(Path.Combine(target, "server-key.pem"), key.ExportPkcs8PrivateKeyPem());
        var active = new ActiveTls(1, $"tls/{bundle}/server-cert.pem", $"tls/{bundle}/server-key.pem");
        var pending = Path.Combine(directory, "active-tls.pending.json");
        File.WriteAllText(pending, JsonSerializer.Serialize(active));
        File.Move(pending, Path.Combine(directory, "active-tls.json"), true);
    }

    public static void RequirePrivateDirectory(string directory)
    {
        RejectReparsePoints(directory);
        var current = WindowsIdentity.GetCurrent().User!;
        var security = new DirectoryInfo(directory).GetAccessControl();
        if (!security.AreAccessRulesProtected || !current.Equals(security.GetOwner(typeof(SecurityIdentifier))))
            throw new UnauthorizedAccessException("Private directory ownership changed.");
        foreach (FileSystemAccessRule rule in security.GetAccessRules(true, true, typeof(SecurityIdentifier)))
        {
            if (rule.AccessControlType == AccessControlType.Allow && !current.Equals(rule.IdentityReference) &&
                !new SecurityIdentifier(WellKnownSidType.LocalSystemSid, null).Equals(rule.IdentityReference))
                throw new UnauthorizedAccessException("Private directory permissions changed.");
        }
        CheckChildren(new DirectoryInfo(directory), current);
    }

    private static void CheckChildren(DirectoryInfo directory, SecurityIdentifier current)
    {
        foreach (var item in directory.EnumerateFileSystemInfos())
        {
            if ((item.Attributes & FileAttributes.ReparsePoint) != 0)
                throw new IOException("Reparse point refused.");
            FileSystemSecurity security = item is DirectoryInfo child ? child.GetAccessControl() : ((FileInfo)item).GetAccessControl();
            foreach (FileSystemAccessRule rule in security.GetAccessRules(true, true, typeof(SecurityIdentifier)))
                if (rule.AccessControlType == AccessControlType.Allow && !current.Equals(rule.IdentityReference) &&
                    !new SecurityIdentifier(WellKnownSidType.LocalSystemSid, null).Equals(rule.IdentityReference))
                    throw new UnauthorizedAccessException("Private file permissions changed.");
            if (item is DirectoryInfo nested) CheckChildren(nested, current);
        }
    }

    private static void CreatePrivateDirectory(string directory)
    {
        RejectReparsePoints(directory);
        var current = WindowsIdentity.GetCurrent().User!;
        var security = new DirectorySecurity();
        security.SetOwner(current);
        security.SetAccessRuleProtection(true, false);
        foreach (var sid in new[] { current, new SecurityIdentifier(WellKnownSidType.LocalSystemSid, null) })
            security.AddAccessRule(new FileSystemAccessRule(sid, FileSystemRights.FullControl,
                InheritanceFlags.ContainerInherit | InheritanceFlags.ObjectInherit, PropagationFlags.None, AccessControlType.Allow));
        new DirectoryInfo(directory).Create(security);
    }

    private static FileStream Lock(string directory) => new(Path.Combine(directory, "certificate.lock"),
        FileMode.OpenOrCreate, FileAccess.ReadWrite, FileShare.None);

    private static void RejectReparsePoints(string directory)
    {
        for (var item = new DirectoryInfo(directory); item != null; item = item.Parent)
            if (item.Exists && (item.Attributes & FileAttributes.ReparsePoint) != 0)
                throw new IOException("Reparse point refused.");
    }

    private static void ValidatePassword(string password)
    {
        if (password.Length < 16) throw new ArgumentException("Password too short.");
    }

    private static void ValidateName(string name)
    {
        if (IPAddress.TryParse(name, out var address))
        {
            var bytes = address.GetAddressBytes();
            if (bytes.Length != 4 || !(bytes[0] == 10 || (bytes[0] == 172 && bytes[1] is >= 16 and <= 31) ||
                (bytes[0] == 192 && bytes[1] == 168)))
                throw new ArgumentException("Use a stable private IPv4 address.");
        }
        else if (name.Length > 253 || name.Equals("localhost", StringComparison.OrdinalIgnoreCase) ||
            name.Split('.').Any(label => label.Length is < 1 or > 63 || label[0] == '-' || label[^1] == '-' ||
                label.Any(c => !char.IsAsciiLetterOrDigit(c) && c != '-')))
            throw new ArgumentException("Use an ASCII DNS hostname.");
    }
}

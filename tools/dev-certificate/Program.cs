using System.Net;
using System.Security.Cryptography;
using System.Security.Cryptography.X509Certificates;

if (args.Length != 1 && (args.Length != 3 || args[1] != "--issuer-directory"))
{
    Console.Error.WriteLine("usage: Eitmad.DevCertificate <output-directory> [--issuer-directory <private-CA-directory>] (CA password on stdin)");
    return 1;
}

var directory = Path.GetFullPath(args[0]);
Directory.CreateDirectory(directory);
var now = DateTimeOffset.UtcNow;

static X509Certificate2 CreateAuthority(string name, DateTimeOffset now)
{
    using var key = ECDsa.Create(ECCurve.NamedCurves.nistP256);
    var request = new CertificateRequest($"CN={name}", key, HashAlgorithmName.SHA256);
    request.CertificateExtensions.Add(new X509BasicConstraintsExtension(true, false, 0, true));
    request.CertificateExtensions.Add(new X509KeyUsageExtension(
        X509KeyUsageFlags.KeyCertSign | X509KeyUsageFlags.CrlSign, true));
    request.CertificateExtensions.Add(new X509SubjectKeyIdentifierExtension(request.PublicKey, false));
    return request.CreateSelfSigned(now.AddMinutes(-5), now.AddDays(7));
}

static X509Certificate2 LoadAuthority(string path)
{
    var password = Console.ReadLine() ?? throw new InvalidOperationException("Missing private CA input.");
    using var key = ECDsa.Create();
    key.ImportFromEncryptedPem(File.ReadAllText(Path.Combine(path, "ca-key.encrypted.pem")), password);
    using var certificate = X509Certificate2.CreateFromPem(File.ReadAllText(Path.Combine(path, "trusted-ca.pem")));
    return certificate.CopyWithPrivateKey(key);
}

using var authority = args.Length == 3 ? LoadAuthority(args[2]) : CreateAuthority("Eitmad direct test CA", now);
using var wrongAuthority = CreateAuthority("Eitmad unrelated test CA", now);
using var serverKey = ECDsa.Create(ECCurve.NamedCurves.nistP256);
var serverRequest = new CertificateRequest("CN=localhost", serverKey, HashAlgorithmName.SHA256);
serverRequest.CertificateExtensions.Add(new X509BasicConstraintsExtension(false, false, 0, true));
serverRequest.CertificateExtensions.Add(new X509KeyUsageExtension(X509KeyUsageFlags.DigitalSignature, true));
serverRequest.CertificateExtensions.Add(new X509EnhancedKeyUsageExtension(
    new OidCollection { new("1.3.6.1.5.5.7.3.1") }, true));
var names = new SubjectAlternativeNameBuilder();
names.AddDnsName("localhost");
names.AddIpAddress(IPAddress.Loopback);
serverRequest.CertificateExtensions.Add(names.Build());
var serial = RandomNumberGenerator.GetBytes(16);
using var issued = serverRequest.Create(authority, now.AddMinutes(-5), now.AddDays(2), serial);
using var server = issued.CopyWithPrivateKey(serverKey);

File.WriteAllText(Path.Combine(directory, "trusted-ca.pem"), authority.ExportCertificatePem());
File.WriteAllText(Path.Combine(directory, "wrong-ca.pem"), wrongAuthority.ExportCertificatePem());
File.WriteAllText(Path.Combine(directory, "server-cert.pem"), server.ExportCertificatePem());
File.WriteAllText(Path.Combine(directory, "server-key.pem"), serverKey.ExportPkcs8PrivateKeyPem());
Console.WriteLine("development certificates created");
return 0;

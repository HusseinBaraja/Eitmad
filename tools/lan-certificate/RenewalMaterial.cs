using System.Net.Security;
using System.Net.Sockets;
using System.Security.AccessControl;
using System.Security.Cryptography;
using System.Security.Cryptography.X509Certificates;
using System.Security.Principal;
using System.Text;
using System.Text.Json;

namespace Eitmad.LanCertificate;

// Windows/cryptographic mechanics only. Rust owns deadlines, transactions, and recovery.
public static class RenewalMaterial
{
    public sealed record Validity(long NotBefore, long NotAfter, long CaNotAfter, string Fingerprint);

    internal static void CreateDirectory(string path)
    {
        var current = WindowsIdentity.GetCurrent();
        var owner = new WindowsPrincipal(current).IsInRole(WindowsBuiltInRole.Administrator)
            ? new SecurityIdentifier(WellKnownSidType.BuiltinAdministratorsSid, null) : current.User!;
        var acl = new DirectorySecurity();
        acl.SetOwner(owner);
        acl.SetAccessRuleProtection(true, false);
        foreach (var sid in new[] { owner, new SecurityIdentifier(WellKnownSidType.LocalSystemSid, null) })
            acl.AddAccessRule(new FileSystemAccessRule(sid, FileSystemRights.FullControl,
                InheritanceFlags.ContainerInherit | InheritanceFlags.ObjectInherit, PropagationFlags.None, AccessControlType.Allow));
        new DirectoryInfo(path).Create(acl);
    }

    public static void RequireControl(string directory)
    {
        for (var item = new DirectoryInfo(Path.GetFullPath(directory)); item != null; item = item.Parent)
            if (item.Exists && (item.Attributes & FileAttributes.ReparsePoint) != 0)
                throw new IOException("Reparse point refused.");
        Check(new DirectoryInfo(directory));
    }

    private static void Check(FileSystemInfo item)
    {
        if ((item.Attributes & FileAttributes.ReparsePoint) != 0) throw new IOException("Reparse point refused.");
        FileSystemSecurity acl = item is DirectoryInfo d ? d.GetAccessControl() : ((FileInfo)item).GetAccessControl();
        var trusted = new[] { WindowsIdentity.GetCurrent().User!,
            new SecurityIdentifier(WellKnownSidType.LocalSystemSid, null),
            new SecurityIdentifier(WellKnownSidType.BuiltinAdministratorsSid, null) };
        if (!trusted.Contains(acl.GetOwner(typeof(SecurityIdentifier)))) throw new UnauthorizedAccessException();
        foreach (FileSystemAccessRule rule in acl.GetAccessRules(true, true, typeof(SecurityIdentifier)))
            if (rule.AccessControlType == AccessControlType.Allow && !trusted.Contains(rule.IdentityReference))
                throw new UnauthorizedAccessException();
        if (item is DirectoryInfo directory)
            foreach (var child in directory.EnumerateFileSystemInfos()) Check(child);
    }

    public static void RetainUnlock(string directory, string password)
    {
        RequireControl(directory);
        // Check the unlock and key match before retaining a credential. Never replace the CA.
        using var key = ECDsa.Create();
        key.ImportFromEncryptedPem(File.ReadAllText(Path.Combine(directory, "ca-key.encrypted.pem")), password);
        using var certificate = X509Certificate2.CreateFromPem(File.ReadAllText(Path.Combine(directory, "trusted-ca.pem")));
        using var ca = certificate.CopyWithPrivateKey(key);
        var bytes = Encoding.UTF8.GetBytes(password);
        try
        {
            var encrypted = ProtectedData.Protect(bytes, null, DataProtectionScope.LocalMachine);
            var pending = Path.Combine(directory, Guid.NewGuid().ToString("N") + ".unlock.pending");
            Certificates.WriteDurable(pending, encrypted);
            File.Move(pending, Path.Combine(directory, "unlock.dpapi"), true);
        }
        finally { CryptographicOperations.ZeroMemory(bytes); }
    }

    public static Certificates.ActiveTls Stage(string directory)
    {
        RequireControl(directory);
        byte[] bytes;
        try { bytes = ProtectedData.Unprotect(File.ReadAllBytes(Path.Combine(directory, "unlock.dpapi")), null, DataProtectionScope.LocalMachine); }
        catch (Exception error) when (error is IOException or CryptographicException)
        { throw new CredentialUnavailableException(); }
        try { return Certificates.Stage(directory, Encoding.UTF8.GetString(bytes)); }
        finally { CryptographicOperations.ZeroMemory(bytes); }
    }

    public static Validity Inspect(string certificate, string key, string authority)
    {
        using var pair = X509Certificate2.CreateFromPemFile(certificate, key);
        using var ca = X509Certificate2.CreateFromPem(File.ReadAllText(authority));
        using var chain = new X509Chain();
        chain.ChainPolicy.TrustMode = X509ChainTrustMode.CustomRootTrust;
        chain.ChainPolicy.CustomTrustStore.Add(ca);
        chain.ChainPolicy.RevocationMode = X509RevocationMode.NoCheck;
        // Rust inspects time bounds even when the current leaf is already expired.
        chain.ChainPolicy.VerificationFlags = X509VerificationFlags.IgnoreNotTimeValid;
        chain.ChainPolicy.ApplicationPolicy.Add(new Oid("1.3.6.1.5.5.7.3.1"));
        if (!chain.Build(pair)) throw new CryptographicException("Retained trust mismatch.");
        return new(new DateTimeOffset(pair.NotBefore.ToUniversalTime()).ToUnixTimeSeconds(),
            new DateTimeOffset(pair.NotAfter.ToUniversalTime()).ToUnixTimeSeconds(),
            new DateTimeOffset(ca.NotAfter.ToUniversalTime()).ToUnixTimeSeconds(),
            pair.GetCertHashString(HashAlgorithmName.SHA256));
    }

    public static async Task Probe(string endpoint, string address, string authority, string fingerprint)
    {
        var uri = new Uri(endpoint);
        if (uri.Scheme != "https" || uri.UserInfo.Length != 0 || uri.AbsolutePath != "/" ||
            uri.Query.Length != 0 || uri.Fragment.Length != 0) throw new ArgumentException();
        using var deadline = new CancellationTokenSource(TimeSpan.FromSeconds(5));
        using var ca = X509Certificate2.CreateFromPem(File.ReadAllText(authority));
        using var client = new TcpClient();
        await client.ConnectAsync(address, uri.Port, deadline.Token);
        using var stream = new SslStream(client.GetStream());
        var chain = new X509ChainPolicy { TrustMode = X509ChainTrustMode.CustomRootTrust, RevocationMode = X509RevocationMode.NoCheck };
        chain.CustomTrustStore.Add(ca);
        await stream.AuthenticateAsClientAsync(new SslClientAuthenticationOptions {
            TargetHost = uri.Host, CertificateChainPolicy = chain
        }, deadline.Token);
        using var peer = new X509Certificate2(stream.RemoteCertificate!);
        if (!peer.GetCertHashString(HashAlgorithmName.SHA256).Equals(fingerprint, StringComparison.Ordinal))
            throw new CryptographicException("Host has not loaded the expected pair.");
        await stream.WriteAsync(Encoding.ASCII.GetBytes($"GET /readyz HTTP/1.1\r\nHost: {uri.Authority}\r\nConnection: close\r\n\r\n"), deadline.Token);
        using var reader = new StreamReader(stream, Encoding.ASCII);
        var line = new StringBuilder();
        var buffer = new char[1];
        while (line.Length < 256)
        {
            if (await reader.ReadAsync(buffer.AsMemory(), deadline.Token) == 0) break;
            line.Append(buffer[0]);
            if (buffer[0] == '\n') break;
        }
        if (!line.ToString().StartsWith("HTTP/1.1 204 ", StringComparison.Ordinal) &&
            !line.ToString().StartsWith("HTTP/1.0 204 ", StringComparison.Ordinal)) throw new IOException("HTTPS readiness failed.");
    }

    public sealed class CredentialUnavailableException : Exception;

    public static async Task<int> Execute(string action, string directory)
    {
        try
        {
            var input = await Console.In.ReadToEndAsync();
            if (input.Length > 16384) throw new ArgumentException();
            using var request = JsonDocument.Parse(input);
            string Get(string name) => request.RootElement.GetProperty(name).GetString() ?? throw new ArgumentException();
            object result = new { };
            switch (action)
            {
                case "retain": RetainUnlock(directory, Get("password")); break;
                case "stage": result = Stage(directory); break;
                case "inspect": result = Inspect(Get("certificate"), Get("privateKey"), Get("authority")); break;
                case "probe": await Probe(Get("endpoint"), Get("address"), Get("authority"), Get("fingerprint")); break;
                default: throw new ArgumentException();
            }
            Console.WriteLine(JsonSerializer.Serialize(new { outcome = "succeeded", result }));
            return 0;
        }
        catch (CredentialUnavailableException)
        {
            Console.WriteLine("{\"outcome\":\"credentialUnavailable\"}");
            return 1;
        }
        catch (Exception)
        {
            // No platform/crypto exception text crosses this private tool boundary.
            Console.WriteLine("{\"outcome\":\"failed\"}");
            return 1;
        }
    }
}

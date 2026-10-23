using System.Diagnostics;
using System.Net;
using System.Net.Security;
using System.Net.Sockets;
using System.Security.AccessControl;
using System.Security.Authentication;
using System.Security.Cryptography;
using System.Security.Cryptography.X509Certificates;
using System.Security.Principal;
using System.Text;
using System.Text.Json;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace Eitmad.LanCertificate.Tests;

[TestClass]
public sealed class UnattendedRenewalTests
{
    private const string Password = "isolated unattended test password";

    [TestMethod]
    public async Task RenewalUsesCurrentLocalListenerWhenInstallationAddressIsUnavailable()
    {
        var root = Path.Combine(Path.GetTempPath(), "eitmad-renewal-route-" + Guid.NewGuid().ToString("N"));
        Certificates.Initialize(root, "workshop.test", Password);
        var identity = File.ReadAllBytes(Path.Combine(root, "identity.json"));
        var ca = Path.Combine(root, "trusted-ca.pem");
        var trust = File.ReadAllBytes(ca);
        var staged = Certificates.Stage(root, Password);
        using var renewedPair = Load(root, staged);
        using var server = new ReadinessServer(renewedPair);
        var endpoint = $"https://workshop.test:{server.Port}/";
        const string installationAddress = "127.0.0.2";
        var fingerprint = Inspect(root, staged).Fingerprint;
        await Assert.ThrowsExactlyAsync<SocketException>(() =>
            RenewalMaterial.Probe(endpoint, installationAddress, ca, fingerprint));

        Directory.CreateDirectory(Path.Combine(root, "app"));
        File.WriteAllText(Path.Combine(root, "app/host-service.xml"),
            $"<service><env name=\"EITMAD_SERVER_LISTEN\" value=\"0.0.0.0:{server.Port}\" /></service>");
        var settings = new { endpoint, address = installationAddress, serverPort = server.Port };
        var manifest = Path.Combine(root, "host.json");
        File.WriteAllText(manifest, JsonSerializer.Serialize(settings));
        var savedSettings = File.ReadAllBytes(manifest);
        var address = await SelectProbeAddress(root);

        await RenewalMaterial.Probe(endpoint, address, ca, fingerprint);
        CollectionAssert.AreEqual(savedSettings, File.ReadAllBytes(manifest));
        CollectionAssert.AreEqual(identity, File.ReadAllBytes(Path.Combine(root, "identity.json")));
        CollectionAssert.AreEqual(trust, File.ReadAllBytes(ca));
    }

    private static async Task<string> SelectProbeAddress(string root)
    {
        const string script = """
            $ErrorActionPreference = 'Stop'
            $request = [Console]::In.ReadToEnd() | ConvertFrom-Json
            Import-Module $request.module -Force -DisableNameChecking
            $settings = Get-Content -LiteralPath (Join-Path $request.root 'host.json') -Raw | ConvertFrom-Json
            & (Get-Module tls-renewal) { param($root, $settings) Get-TlsProbeAddress $root $settings } $request.root $settings
            """;
        var start = new ProcessStartInfo(Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.System),
            "WindowsPowerShell/v1.0/powershell.exe")) {
            UseShellExecute = false, CreateNoWindow = true,
            RedirectStandardInput = true, RedirectStandardOutput = true, RedirectStandardError = true
        };
        foreach (var argument in new[] { "-NoProfile", "-NonInteractive", "-EncodedCommand",
            Convert.ToBase64String(Encoding.Unicode.GetBytes(script)) }) start.ArgumentList.Add(argument);
        using var process = Process.Start(start)!;
        var output = process.StandardOutput.ReadToEndAsync();
        var error = process.StandardError.ReadToEndAsync();
        await process.StandardInput.WriteAsync(JsonSerializer.Serialize(new {
            root, module = Path.Combine(AppContext.BaseDirectory, "deploy/windows/tls-renewal.psm1")
        }));
        process.StandardInput.Close();
        using var timeout = new CancellationTokenSource(TimeSpan.FromSeconds(15));
        try { await process.WaitForExitAsync(timeout.Token); }
        catch (OperationCanceledException) { process.Kill(true); throw; }
        Assert.AreEqual(0, process.ExitCode, await error);
        return (await output).Trim();
    }

    [TestMethod]
    public async Task StagedRenewalRequiresNewLeafReadinessAndRetainsClientTrustAcrossReconnect()
    {
        var root = Path.Combine(Path.GetTempPath(), "eitmad-renewal-" + Guid.NewGuid().ToString("N"));
        Certificates.Initialize(root, "workshop.test", Password);
        var original = File.ReadAllText(Path.Combine(root, "active-tls.json"));
        var first = JsonSerializer.Deserialize<Certificates.ActiveTls>(original)!;
        var trust = File.ReadAllBytes(Path.Combine(root, "trusted-ca.pem"));
        RenewalMaterial.RetainUnlock(root, Password);
        var staged = RenewalMaterial.Stage(root);
        Assert.AreEqual(original, File.ReadAllText(Path.Combine(root, "active-tls.json")), "Staging must not activate.");
        using var oldPair = Load(root, first);
        using var newPair = Load(root, staged);
        var oldValidity = Inspect(root, first);
        var newValidity = Inspect(root, staged);
        Assert.AreNotEqual(oldValidity.Fingerprint, newValidity.Fingerprint);
        using var server = new ReadinessServer(oldPair);
        var endpoint = $"https://workshop.test:{server.Port}/";
        var ca = Path.Combine(root, "trusted-ca.pem");
        await RenewalMaterial.Probe(endpoint, "127.0.0.1", ca, oldValidity.Fingerprint);
        await Assert.ThrowsExactlyAsync<CryptographicException>(() => RenewalMaterial.Probe(endpoint, "127.0.0.1", ca, newValidity.Fingerprint));
        // A failed activation can reconnect to the previous pair using the retained client CA.
        await RenewalMaterial.Probe(endpoint, "127.0.0.1", ca, oldValidity.Fingerprint);
        server.Pair = newPair;
        await RenewalMaterial.Probe(endpoint, "127.0.0.1", ca, newValidity.Fingerprint);
        await RenewalMaterial.Probe(endpoint, "127.0.0.1", ca, newValidity.Fingerprint);
        await Assert.ThrowsExactlyAsync<AuthenticationException>(() => RenewalMaterial.Probe(
            $"https://wrong.test:{server.Port}/", "127.0.0.1", ca, newValidity.Fingerprint));
        server.Ready = false;
        await Assert.ThrowsExactlyAsync<IOException>(() => RenewalMaterial.Probe(endpoint, "127.0.0.1", ca, newValidity.Fingerprint));
        server.Ready = true;
        var wrong = Path.Combine(Path.GetTempPath(), "eitmad-renewal-wrong-" + Guid.NewGuid().ToString("N"));
        Certificates.Initialize(wrong, "workshop.test", Password);
        await Assert.ThrowsExactlyAsync<AuthenticationException>(() => RenewalMaterial.Probe(endpoint, "127.0.0.1",
            Path.Combine(wrong, "trusted-ca.pem"), newValidity.Fingerprint));
        CollectionAssert.AreEqual(trust, File.ReadAllBytes(ca));
        File.WriteAllBytes(Path.Combine(root, "unlock.dpapi"), [1, 2, 3]);
        Assert.ThrowsExactly<RenewalMaterial.CredentialUnavailableException>(() => RenewalMaterial.Stage(root));
        Assert.AreEqual(original, File.ReadAllText(Path.Combine(root, "active-tls.json")));
        // Credential loss does not remove the last working key or break existing client trust.
        await RenewalMaterial.Probe(endpoint, "127.0.0.1", ca, newValidity.Fingerprint);
    }

    [TestMethod]
    public void BroadenedUnlockAccessAndMismatchedKeyAreRefused()
    {
        var root = Path.Combine(Path.GetTempPath(), "eitmad-renewal-acl-" + Guid.NewGuid().ToString("N"));
        Certificates.Initialize(root, "workshop.test", Password);
        RenewalMaterial.RetainUnlock(root, Password);
        var unlock = new FileInfo(Path.Combine(root, "unlock.dpapi"));
        var acl = unlock.GetAccessControl();
        var broad = new FileSystemAccessRule(new SecurityIdentifier(WellKnownSidType.BuiltinUsersSid, null), FileSystemRights.Read, AccessControlType.Allow);
        acl.AddAccessRule(broad); unlock.SetAccessControl(acl);
        try { Assert.ThrowsExactly<UnauthorizedAccessException>(() => RenewalMaterial.Stage(root)); }
        finally { acl.RemoveAccessRuleSpecific(broad); unlock.SetAccessControl(acl); }
        var first = JsonSerializer.Deserialize<Certificates.ActiveTls>(File.ReadAllText(Path.Combine(root, "active-tls.json")))!;
        var next = RenewalMaterial.Stage(root);
        Assert.ThrowsExactly<CryptographicException>(() => RenewalMaterial.Inspect(Path.Combine(root, first.Certificate),
            Path.Combine(root, next.PrivateKey), Path.Combine(root, "trusted-ca.pem")));
        Assert.ThrowsExactly<CryptographicException>(() => RenewalMaterial.RetainUnlock(root, "incorrect retained CA password"));
        Assert.IsNotNull(RenewalMaterial.Stage(root));
    }

    private static X509Certificate2 Load(string root, Certificates.ActiveTls pair) =>
        X509Certificate2.CreateFromPemFile(Path.Combine(root, pair.Certificate), Path.Combine(root, pair.PrivateKey));

    private static RenewalMaterial.Validity Inspect(string root, Certificates.ActiveTls pair) =>
        RenewalMaterial.Inspect(Path.Combine(root, pair.Certificate), Path.Combine(root, pair.PrivateKey), Path.Combine(root, "trusted-ca.pem"));

    private sealed class ReadinessServer : IDisposable
    {
        private readonly TcpListener listener = new(IPAddress.Loopback, 0);
        private readonly CancellationTokenSource stop = new();
        private readonly Task serving;
        public X509Certificate2 Pair { get; set; }
        public bool Ready { get; set; } = true;
        public int Port => ((IPEndPoint)listener.LocalEndpoint).Port;

        public ReadinessServer(X509Certificate2 pair)
        {
            Pair = pair;
            listener.Start();
            serving = Serve();
        }

        private async Task Serve()
        {
            while (!stop.IsCancellationRequested)
            {
                try
                {
                    using var client = await listener.AcceptTcpClientAsync(stop.Token);
                    using var stream = new SslStream(client.GetStream());
                    // SChannel requires a persisted native key for server-side authentication.
                    using var native = new X509Certificate2(Pair.Export(X509ContentType.Pkcs12), (string?)null,
                        X509KeyStorageFlags.DefaultKeySet);
                    await stream.AuthenticateAsServerAsync(new SslServerAuthenticationOptions { ServerCertificate = native }, stop.Token);
                    var buffer = new byte[4096];
                    if (await stream.ReadAsync(buffer, stop.Token) == 0) continue;
                    var code = Ready ? "204 No Content" : "503 Service Unavailable";
                    await stream.WriteAsync(Encoding.ASCII.GetBytes($"HTTP/1.1 {code}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"), stop.Token);
                }
                catch (Exception error) when (error is IOException or AuthenticationException or OperationCanceledException or SocketException) { }
            }
        }

        public void Dispose()
        {
            stop.Cancel(); listener.Stop();
            serving.GetAwaiter().GetResult();
            stop.Dispose();
        }
    }
}

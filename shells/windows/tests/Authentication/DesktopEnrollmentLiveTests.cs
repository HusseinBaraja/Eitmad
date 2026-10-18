using System.Text.Json;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;
using Eitmad.Platform.Windows.Shell;
using Eitmad.WindowsShell.Features.Authentication;

namespace Eitmad.WindowsShell.Tests.Authentication;

[TestClass]
[DoNotParallelize]
public sealed class DesktopEnrollmentLiveTests
{
    [TestMethod]
    [TestCategory("DesktopEnrollmentEnvironment")]
    public async Task FreshDesktopEnrollsReopensAndReauthenticates()
    {
        var input = Environment.GetEnvironmentVariable("EITMAD_DESKTOP_ENROLLMENT_INPUT");
        if (string.IsNullOrEmpty(input)) Assert.Inconclusive("Run the Rust desktop enrollment integration test with EITMAD_DESKTOP_ENROLLMENT_ENGINE set.");
        var connection = JsonSerializer.Deserialize<DesktopEnrollmentSettings>(input!)!;
        var arguments = new[] { "--engine", Environment.GetEnvironmentVariable("EITMAD_DESKTOP_ENROLLMENT_ENGINE")!,
            "--runtime-directory", Environment.GetEnvironmentVariable("EITMAD_DESKTOP_ENROLLMENT_DIRECTORY")! };
        Guid? device;
        Guid tenant;
        await using (var sessions = new DesktopSessionController(WindowsEngineBridge.Create(arguments)))
        {
            await sessions.StartAsync();
            Assert.AreEqual(AuthenticatedSurface.Manager, await sessions.SignInAsync("owner", "synthetic-test-password-123", connection));
            await sessions.SignOutAsync();
            await sessions.StopAsync();
        }
        var reopened = WindowsEngineBridge.Create(arguments);
        await using (var sessions = new DesktopSessionController(reopened))
        {
            await sessions.StartAsync();
            Assert.AreEqual(AuthenticatedSurface.Manager, await sessions.SignInAsync("owner", "synthetic-test-password-123", null));
            var state = await reopened.GetSessionStateAsync();
            Assert.IsNotNull(state?.Authorization);
            device = state.Authorization.Identity.DeviceId;
            tenant = state.Authorization.TenantId;
            await sessions.SignOutAsync();
            await Assert.ThrowsAsync<EngineIpcException>(() => sessions.SignInAsync("owner", "wrong-synthetic-password", null));
            Assert.IsNull(await reopened.GetSessionStateAsync());
            Assert.AreEqual(AuthenticatedSurface.Manager, await sessions.SignInAsync("owner", "synthetic-test-password-123", null));
            var recovered = await reopened.GetSessionStateAsync();
            Assert.AreEqual(device, recovered!.Authorization.Identity.DeviceId);
            Assert.AreEqual(tenant, recovered.Authorization.TenantId);
            await sessions.StopAsync();
        }
    }
}

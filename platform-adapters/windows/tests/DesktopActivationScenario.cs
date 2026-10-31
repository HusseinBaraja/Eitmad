using System.Runtime.InteropServices;
using System.Runtime.Versioning;
using Eitmad.Platform.Windows.Installation;

[SupportedOSPlatform("windows")]
internal static class DesktopActivationScenario
{
    internal static async Task UnregisteredApplicationReportsWindowsFailure()
    {
        try
        {
            await Task.Run(() => WindowsDesktopActivation.Activate(
                "Eitmad.Missing." + Guid.NewGuid().ToString("N") + "_0000000000000!Desktop"));
        }
        catch (IOException error)
        {
            Assert.True(error.InnerException is COMException { HResult: < 0 },
                "activation retains the native failure instead of accepting a helper exit");
            return;
        }
        throw new InvalidOperationException("An unregistered application must not report successful activation.");
    }
}

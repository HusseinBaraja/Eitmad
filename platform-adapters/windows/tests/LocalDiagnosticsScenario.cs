using System.IO.Compression;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.Diagnostics;

internal static class LocalDiagnosticsScenario
{
    public static async Task RotationAndReportExcludePrivateOrDamagedInput()
    {
        var root = Path.Combine(Path.GetTempPath(), "eitmad-diagnostic-check-" + Guid.NewGuid().ToString("N"));
        var logs = Path.Combine(root, "logs");
        var destination = Path.Combine(root, "downloads");
        Directory.CreateDirectory(logs);
        try
        {
            var correlation = Guid.NewGuid();
            foreach (var details in new[] { "{\"hresult\":\"private-password-sentinel\",\"line\":{}}", "[1,2,3]", "{truncated" })
                LocalDiagnostics.RecordNativeSetupFailure(new WindowsInstallProgress {
                    Version = 1, Sequence = 1, Stage = WindowsInstallStage.Preflight, Status = WindowsInstallStatus.Failed, Details = details
                });
            using (var recorder = new LocalDiagnosticRecorder(logs, LocalDiagnosticComponent.Desktop))
            {
                var error = new IOException("private-password-sentinel", new System.ComponentModel.Win32Exception(5, "private-customer-sentinel"));
                var projected = LocalDiagnostics.ExceptionEvent("Crash", error, correlation);
                Assert.Equal(5L, projected.NativeCode!.Value, "underlying Windows cause survives the safe projection");
                Assert.False(JsonSerializer.Serialize(projected).Contains("private-", StringComparison.Ordinal), "exception text is excluded");
                // Enough metadata to cross the real retention bound, without changing production policy.
                for (var index = 0; index < 3200; index++)
                {
                    recorder.Record(new LocalDiagnosticEvent { Phase = "IpcReply", Result = "Failed", Correlation = correlation,
                        ErrorCode = ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1,
                        Locations = Enumerable.Repeat("SyntheticMethod." + new string('x', 220), 12).ToArray() });
                    if (index % 64 == 63) await recorder.FlushAsync();
                }
                await recorder.FlushAsync();
                Assert.Equal(0L, recorder.Failures, "valid records persist");
                Assert.Equal(0L, recorder.Dropped, "batched records retain their full sequence");
            }
            var files = Directory.GetFiles(logs, "*.jsonl");
            Assert.Equal(4, files.Length, "rotation retains four files");
            Assert.True(files.All(file => new FileInfo(file).Length <= 2 * 1024 * 1024), "each diagnostic file is bounded");
            var last = files.OrderByDescending(File.GetLastWriteTimeUtc).First();
            var lines = File.ReadAllLines(last);
            foreach (var component in new object[] { "private-component-sentinel", 42 })
            {
                var damaged = System.Text.Json.Nodes.JsonNode.Parse(lines[0])!;
                damaged["component"] = JsonSerializer.SerializeToNode(component);
                File.AppendAllText(last, damaged.ToJsonString() + "\n");
            }
            var valid = JsonSerializer.Deserialize<LocalDiagnosticEvent>(lines[0], Converter.Settings)!;
            valid.ErrorCode = "private-customer-sentinel";
            File.AppendAllText(last, JsonSerializer.Serialize(valid, Converter.Settings) + "\n{\"password\":\"private-password-sentinel\"}\n{broken-tail");
            File.WriteAllText(Path.Combine(logs, "database.db"), "private-database-sentinel");
            var report = DiagnosticReport.Save(destination, logs, Path.Combine(root, "server-missing"));
            using var zip = ZipFile.OpenRead(report);
            Assert.Equal(3, zip.Entries.Count, "only typed evidence, manifest and guidance are exported");
            using var eventReader = new StreamReader(zip.GetEntry("events.jsonl")!.Open());
            var text = await eventReader.ReadToEndAsync();
            Assert.False(text.Contains("private-", StringComparison.Ordinal), "raw, unknown and malformed records never enter the report");
            var records = text.Split('\n', StringSplitOptions.RemoveEmptyEntries).Select(line => JsonSerializer.Deserialize<LocalDiagnosticEvent>(line, Converter.Settings)!).ToArray();
            Assert.True(records.Length > 0 && records.All(record => record.Correlation == correlation), "failure correlation survives rotation and export");
            Assert.True(records.All(record => record.BinarySha256.Length == 64 && record.NativeCode is null), "binary identity is recorded without invented native errors");
            Assert.True(records.Zip(records.Skip(1)).All(pair => pair.First.UtcMillis <= pair.Second.UtcMillis), "report timeline is ordered");
            using var manifestReader = new StreamReader(zip.GetEntry("manifest.json")!.Open());
            var manifest = JsonSerializer.Deserialize<LocalDiagnosticManifest>(await manifestReader.ReadToEndAsync(), Converter.Settings)!;
            Assert.True(manifest.Sources.Single(source => source.Component == LocalDiagnosticComponent.Desktop).RejectedRecords >= 5, "damaged evidence is reported");
            Assert.Equal("Missing", manifest.Sources.Single(source => source.Component == LocalDiagnosticComponent.Server).Status, "missing server evidence is explicit");
            Assert.Equal(Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(text))).ToLowerInvariant(), manifest.EventsSha256, "timeline checksum matches the ZIP");
            Assert.False(Directory.GetFiles(destination).Any(file => file.EndsWith(".partial", StringComparison.Ordinal)), "successful export leaves no temporary archive");
        }
        finally { Directory.Delete(root, recursive: true); }
    }
}

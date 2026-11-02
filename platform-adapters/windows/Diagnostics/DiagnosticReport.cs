using System.IO.Compression;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using Eitmad.Contracts;

namespace Eitmad.Platform.Windows.Diagnostics;

/// <summary>Explicit local export. Reads only bounded typed diagnostic records, never raw service logs or state directories.</summary>
public static class DiagnosticReport
{
    private static long flushFailures;
    /// <summary>Flush current-process evidence, then export off the caller thread to the Windows Downloads known folder.
    /// A flush timeout is counted and capture continues. Folder lookup, access and I/O failures propagate to the caller.</summary>
    public static async Task<string> SaveToDownloadsAsync()
    {
        LocalDiagnostics.Record("Export", "Started");
        try { await LocalDiagnostics.FlushAsync().ConfigureAwait(false); }
        catch (OperationCanceledException) { Interlocked.Increment(ref flushFailures); LocalDiagnostics.Record("Capture", "Unavailable"); }
        return await Task.Run(() => Save(Downloads(), LocalDiagnostics.UserDirectory, LocalDiagnostics.ServerDirectory)).ConfigureAwait(false);
    }

    /// <summary>Read only bounded typed sources, count rejected records and unavailable evidence, and sort by UTC/session/sequence.
    /// Reparse paths are refused. Write a new ZIP through an exclusive partial file; remove that partial file if writing fails.
    /// The manifest describes source statuses, rejected records, omitted/oversized files, capture failures, pending drops and the timeline hash.</summary>
    public static string Save(string destinationDirectory, string userDirectory, string serverDirectory)
    {
        LocalDiagnosticRecorder.PlainPath(destinationDirectory);
        Directory.CreateDirectory(destinationDirectory);
        var name = "Eitmad-Logs-" + DateTime.UtcNow.ToString("yyyyMMdd-HHmmss") + "-" + Guid.NewGuid().ToString("N")[..8] + ".zip";
        var destination = Path.Combine(destinationDirectory, name);
        var temporary = destination + ".partial";
        var events = new List<LocalDiagnosticEvent>();
        var manifest = new LocalDiagnosticManifest {
            Version = 1, ExportedUtcMillis = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(),
            CaptureFailures = LocalDiagnostics.CaptureFailures + Interlocked.Read(ref flushFailures), PendingDroppedEvents = LocalDiagnostics.PendingDropped,
            Services = WindowsServiceDiagnostics.Read(),
        };
        var sources = new List<LocalDiagnosticSource>();
        foreach (var component in Enum.GetValues<LocalDiagnosticComponent>())
            sources.Add(ReadSource(component == LocalDiagnosticComponent.Server ? serverDirectory : userDirectory, component, events));
        manifest.Sources = sources.ToArray();
        events.Sort((left, right) => {
            var order = left.UtcMillis.CompareTo(right.UtcMillis);
            if (order == 0) order = left.Session.CompareTo(right.Session);
            return order != 0 ? order : left.Sequence.CompareTo(right.Sequence);
        });
        try
        {
            using (var file = new FileStream(temporary, FileMode.CreateNew, FileAccess.Write, FileShare.None))
            using (var archive = new ZipArchive(file, ZipArchiveMode.Create))
            {
                using var digest = IncrementalHash.CreateHash(HashAlgorithmName.SHA256);
                using (var output = archive.CreateEntry("events.jsonl", CompressionLevel.Fastest).Open())
                {
                    foreach (var record in events)
                    {
                        var bytes = Encoding.UTF8.GetBytes(JsonSerializer.Serialize(record, LocalDiagnosticRecorder.Json) + "\n");
                        output.Write(bytes); digest.AppendData(bytes);
                    }
                }
                manifest.EventsSha256 = Convert.ToHexString(digest.GetHashAndReset()).ToLowerInvariant();
                using (var output = archive.CreateEntry("manifest.json", CompressionLevel.Fastest).Open())
                    JsonSerializer.Serialize(output, manifest, LocalDiagnosticRecorder.Json);
                using var guide = new StreamWriter(archive.CreateEntry("READ-ME.txt").Open(), new UTF8Encoding(false));
                guide.Write("Eitmad local diagnostic report v1\n"
                    + "Start with manifest.json: check missing/denied sources, rejected/truncated records, capture failures and dropped events.\n"
                    + "events.jsonl is ordered by UTC milliseconds, session and sequence. Clocks on different PCs can differ.\n"
                    + "Match binarySha256/build, correlation and operation before proposing a cause. A missing event is not proof of success.\n"
                    + "IpcReply Succeeded proves response delivery. Match the engine event by correlation for the product outcome.\n"
                    + "Process Started without Stopped can mean a crash, power loss, a running process, or rotation. Do not claim which without evidence.\n"
                    + "Stack symbols and nativeCode identify the failing code path. Messages, arguments, customer content, credentials, database/config files and raw logs are excluded.\n"
                    + "Server events are local to the server PC. Collect a report there too for a server-side failure.\n"
                    + "This is diagnostic evidence, not a tamper-proof audit. eventsSha256 detects copy damage, not a malicious edit.\n");
            }
            File.Move(temporary, destination);
            LocalDiagnostics.Record("Export", "Succeeded");
            return destination;
        }
        catch
        {
            // This path was created by this export with exclusive creation.
            if (File.Exists(temporary)) File.Delete(temporary);
            throw;
        }
    }

    private static LocalDiagnosticSource ReadSource(string directory, LocalDiagnosticComponent component, List<LocalDiagnosticEvent> output)
    {
        var result = new LocalDiagnosticSource { Component = component, Status = "Missing" };
        try
        {
            LocalDiagnosticRecorder.PlainPath(directory);
            var root = new DirectoryInfo(directory);
            if (!root.Exists) return result;
            var prefix = component.ToString().ToLowerInvariant() + "-";
            var files = root.GetFiles(prefix + "*.jsonl").OrderByDescending(file => file.LastWriteTimeUtc).ToArray();
            result.TruncatedFiles = Math.Max(0, files.Length - LocalDiagnosticRecorder.Limit("filesPerComponent"));
            foreach (var file in files.Take(LocalDiagnosticRecorder.Limit("filesPerComponent")))
            {
                LocalDiagnosticRecorder.PlainPath(file.FullName);
                using var input = new FileStream(file.FullName, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete);
                var count = (int)Math.Min(input.Length, LocalDiagnosticRecorder.Limit("fileBytes"));
                var bytes = new byte[count];
                var length = 0;
                while (length < count) { var read = input.Read(bytes, length, count - length); if (read == 0) break; length += read; }
                if (input.Length > count) result.TruncatedFiles++;
                var start = 0;
                for (var end = 0; end < length; end++)
                {
                    if (bytes[end] != (byte)'\n') continue;
                    try
                    {
                        if (end - start >= LocalDiagnosticRecorder.Limit("recordBytes")) throw new JsonException();
                        var record = JsonSerializer.Deserialize<LocalDiagnosticEvent>(bytes.AsSpan(start, end - start), LocalDiagnosticRecorder.Json);
                        if (record is null || record.Component != component || !LocalDiagnosticRecorder.Valid(record)) throw new JsonException();
                        output.Add(record); result.Records++;
                    }
                    // Generated enum converters use Exception for unknown strings and InvalidOperationException for wrong JSON kinds.
                    catch (Exception error) when (error is JsonException or ArgumentException or InvalidOperationException
                        || error.GetType() == typeof(Exception)) { result.RejectedRecords++; }
                    start = end + 1;
                }
                if (start < length) result.RejectedRecords++;
            }
            result.Status = files.Length == 0 ? "Missing" : "Ready";
        }
        catch (UnauthorizedAccessException) { result.Status = "Denied"; }
        catch (IOException) { result.Status = "Unavailable"; }
        return result;
    }

    private static string Downloads()
    {
        if (!OperatingSystem.IsWindows()) throw new PlatformNotSupportedException();
        var code = SHGetKnownFolderPath(new Guid("374DE290-123F-4565-9164-39C4925E467B"), 0, IntPtr.Zero, out var value);
        Marshal.ThrowExceptionForHR(code);
        try { return Marshal.PtrToStringUni(value) ?? throw new IOException("Downloads unavailable."); }
        finally { Marshal.FreeCoTaskMem(value); }
    }

    [DllImport("shell32.dll", ExactSpelling = true)]
    private static extern int SHGetKnownFolderPath([MarshalAs(UnmanagedType.LPStruct)] Guid id, uint flags, IntPtr token, out IntPtr path);
}

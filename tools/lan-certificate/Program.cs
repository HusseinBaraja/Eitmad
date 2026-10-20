using Eitmad.LanCertificate;

if (args is not ["initialize", _, _] and not ["renew", _])
{
    Console.Error.WriteLine("usage: Eitmad.LanCertificate initialize <new-directory> <DNS-name-or-IPv4> | renew <directory>");
    return 1;
}

try
{
    Console.Write("CA password (at least 16 characters; retain it for renewal): ");
    string password;
    if (Console.IsInputRedirected)
        password = Console.ReadLine() ?? "";
    else
    {
        var input = new System.Text.StringBuilder();
        ConsoleKeyInfo key;
        while ((key = Console.ReadKey(true)).Key != ConsoleKey.Enter)
        {
            if (key.Key == ConsoleKey.Backspace && input.Length > 0) input.Length--;
            else if (!char.IsControl(key.KeyChar)) input.Append(key.KeyChar);
        }
        Console.WriteLine();
        password = input.ToString();
        input.Clear();
    }
    if (args[0] == "initialize") Certificates.Initialize(args[1], args[2], password);
    else Certificates.Renew(args[1], password);
    Console.WriteLine("TLS bundle activated. Restart the server to load it. Client CA trust is unchanged by renewal.");
    Console.WriteLine($"CA SHA-256: {Certificates.Fingerprint(args[1])}");
    return 0;
}
catch (Exception error) when (error is ArgumentException or IOException or
    UnauthorizedAccessException or System.Security.Cryptography.CryptographicException or InvalidOperationException)
{
    // Crypto and I/O exception text can contain private paths. Keep diagnostics bounded.
    Console.Error.WriteLine("Certificate operation failed. Check the name, password, expiry, directory ownership, and existing state.");
    return 1;
}

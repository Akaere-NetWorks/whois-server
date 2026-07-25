using System.Net.Security;
using System.Net.Sockets;
using System.Security.Cryptography.X509Certificates;
using System.Text;

namespace WhoisServer.Services.NetworkTools;

/// <summary>Ported from src/services/ssl.rs — the <c>-SSL</c> suffix: connects, completes a TLS
/// handshake, and reports the leaf certificate's details. Uses a permissive
/// <see cref="RemoteCertificateValidationCallback"/> (always returns true) since the goal is to
/// *inspect* the certificate presented, including expired/invalid ones, not to validate trust —
/// the direct analog of the Rust version's <c>AcceptAllVerifier</c>.</summary>
public sealed class SslService
{
    private static readonly TimeSpan Timeout = TimeSpan.FromSeconds(10);

    public async Task<string> ProcessAsync(string query, CancellationToken ct = default)
    {
        var (domain, port) = ParseQuery(query);

        try
        {
            using var tcpClient = new TcpClient();
            using var timeoutCts = CancellationTokenSource.CreateLinkedTokenSource(ct);
            timeoutCts.CancelAfter(Timeout);
            await tcpClient.ConnectAsync(domain, port, timeoutCts.Token);

#pragma warning disable CA5359 // intentional: this service inspects whatever certificate is presented (including expired/self-signed/untrusted ones), it does not use the connection to transfer trust-sensitive data
            using var sslStream = new SslStream(tcpClient.GetStream(), leaveInnerStreamOpen: false,
                (_, _, _, _) => true);
#pragma warning restore CA5359

            await sslStream.AuthenticateAsClientAsync(new SslClientAuthenticationOptions
            {
                TargetHost = domain,
            }, timeoutCts.Token);

            var cert = sslStream.RemoteCertificate as X509Certificate2
                ?? (sslStream.RemoteCertificate is not null ? new X509Certificate2(sslStream.RemoteCertificate) : null);

            if (cert is null)
            {
                return $"SSL Certificate Query Failed for {domain}:{port}\nError: No certificate presented\n";
            }

            return FormatCertificate(cert, domain, port);
        }
        catch (Exception e)
        {
            return $"SSL Certificate Query Failed for {domain}:{port}\nError: {e.Message}\n";
        }
    }

    private static (string Domain, int Port) ParseQuery(string query)
    {
        var colonIndex = query.LastIndexOf(':');
        if (colonIndex >= 0 && int.TryParse(query[(colonIndex + 1)..], out var port))
        {
            return (query[..colonIndex], port);
        }

        return (query, 443);
    }

    private static string FormatCertificate(X509Certificate2 cert, string domain, int port)
    {
        var sb = new StringBuilder();
        sb.Append($"SSL Certificate Information for {domain}:{port}\n");
        sb.Append(new string('=', 60)).Append('\n');

        sb.Append($"Subject: {cert.Subject}\n");
        sb.Append($"Issuer: {cert.Issuer}\n");
        sb.Append($"Serial Number: {cert.SerialNumber}\n");
        sb.Append($"Version: {cert.Version}\n\n");

        sb.Append("Validity Period:\n");
        sb.Append($"  Not Before: {cert.NotBefore:yyyy-MM-dd HH:mm:ss} UTC\n");
        sb.Append($"  Not After: {cert.NotAfter:yyyy-MM-dd HH:mm:ss} UTC\n\n");

        sb.Append("Algorithms:\n");
        sb.Append($"  Signature Algorithm: {cert.SignatureAlgorithm.FriendlyName}\n");
        sb.Append($"  Public Key Algorithm: {cert.PublicKey.Oid.FriendlyName}\n\n");

        var sanExtension = cert.Extensions["2.5.29.17"];
        if (sanExtension is not null)
        {
            sb.Append("Subject Alternative Names:\n");
            sb.Append($"  {sanExtension.Format(true).Replace("\n", "\n  ")}\n\n");
        }

        var now = DateTime.UtcNow;
        var isValid = now >= cert.NotBefore.ToUniversalTime() && now <= cert.NotAfter.ToUniversalTime();
        sb.Append("Certificate Properties:\n");
        sb.Append($"  Currently Valid (by date): {(isValid ? "Yes" : "No")}\n");
        sb.Append($"  Is Self-Signed: {(cert.Subject == cert.Issuer ? "Yes" : "No")}\n\n");

        sb.Append("Fingerprints:\n");
        sb.Append($"  SHA1: {cert.GetCertHashString(System.Security.Cryptography.HashAlgorithmName.SHA1)}\n");
        sb.Append($"  SHA256: {cert.GetCertHashString(System.Security.Cryptography.HashAlgorithmName.SHA256)}\n");

        return sb.ToString();
    }
}

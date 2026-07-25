using System.Net.Sockets;
using System.Text;
using Microsoft.Extensions.Logging;

namespace WhoisServer.Services.Whois;

/// <summary>
/// Raw RFC 3912 WHOIS TCP client — ported from src/services/whois.rs's <c>query_whois</c>.
/// Connection-per-query, CRLF-terminated request, read until EOF/timeout/1MB cap.
/// </summary>
public sealed class WhoisClient(ILogger<WhoisClient> logger)
{
    private const int MaxResponseBytes = 1_000_000;

    public async Task<string> QueryAsync(string query, string server, int port, TimeSpan timeout, CancellationToken ct = default)
    {
        using var cts = CancellationTokenSource.CreateLinkedTokenSource(ct);
        cts.CancelAfter(timeout);

        using var client = new TcpClient();
        try
        {
            await client.ConnectAsync(server, port, cts.Token);
        }
        catch (OperationCanceledException) when (!ct.IsCancellationRequested)
        {
            throw new TimeoutException($"Connection to WHOIS server timed out: {server}:{port}");
        }
        catch (SocketException e)
        {
            throw new IOException($"Cannot connect to WHOIS server {server}:{port}: {e.Message}", e);
        }

        client.NoDelay = true;

        await using var stream = client.GetStream();
        var requestBytes = Encoding.UTF8.GetBytes($"{query}\r\n");

        try
        {
            await stream.WriteAsync(requestBytes, cts.Token);
        }
        catch (OperationCanceledException) when (!ct.IsCancellationRequested)
        {
            throw new TimeoutException("Query write to WHOIS server timed out");
        }

        var response = new StringBuilder();
        var buffer = new byte[8192];
        var totalBytes = 0;

        try
        {
            while (true)
            {
                var read = await stream.ReadAsync(buffer, cts.Token);
                if (read == 0)
                {
                    break;
                }

                response.Append(Encoding.UTF8.GetString(buffer, 0, read));
                totalBytes += read;

                if (totalBytes > MaxResponseBytes)
                {
                    logger.LogDebug("Response from {Server} exceeded size limit, truncating", server);
                    break;
                }
            }
        }
        catch (OperationCanceledException) when (!ct.IsCancellationRequested)
        {
            // Matches the Rust version: a read timeout returns whatever was read so far rather
            // than failing the whole query, as long as something was actually read.
            logger.LogDebug("Timeout reading WHOIS response from {Server} after {Bytes} bytes", server, totalBytes);
        }

        logger.LogDebug("Received {Bytes} bytes from {Server}:{Port}", totalBytes, server, port);

        if (response.Length == 0)
        {
            throw new IOException($"Empty response from WHOIS server {server}:{port}");
        }

        return response.ToString();
    }
}

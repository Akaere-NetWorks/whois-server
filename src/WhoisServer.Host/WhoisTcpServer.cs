using System.Net;
using System.Net.Sockets;
using System.Text;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using WhoisServer.Core.Color;
using WhoisServer.Core.Query;
using WhoisServer.Core.Stats;
#if FEATURE_TELEMETRY
using System.Diagnostics;
using WhoisServer.Services.Telemetry;
#endif

namespace WhoisServer.Host;

/// <summary>
/// The RFC 3912 WHOIS TCP listener — ported from src/server/async_server.rs (accept loop +
/// semaphore-gated concurrency limiting) and src/server/connection.rs (per-connection framing).
/// Plain <see cref="TcpListener"/> rather than a Kestrel <c>ConnectionHandler</c> (plan.md §2 —
/// either is a valid choice; this is the simpler one to stand up without pulling in ASP.NET Core
/// before the web dashboard phase actually needs it).
/// </summary>
public sealed class WhoisTcpServer(
    CliOptions options,
    QueryProcessor queryProcessor,
    StatsManager stats,
#if FEATURE_TELEMETRY
    TelemetryService telemetryService,
#endif
    ILogger<WhoisTcpServer> logger) : BackgroundService
{
    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        if (options.DumpTraffic)
        {
            Directory.CreateDirectory(options.DumpDir);
        }

        var listener = new TcpListener(IPAddress.Parse(options.Host), options.Port);
        listener.Start();
        logger.LogInformation("WHOIS server listening on {Host}:{Port}", options.Host, options.Port);

        using var semaphore = new SemaphoreSlim(options.MaxConnections, options.MaxConnections);

        try
        {
            while (!stoppingToken.IsCancellationRequested)
            {
                // Matches the Rust version: the accept loop itself blocks on permit acquisition,
                // so a saturated connection pool throttles new inbound accepts (not an in-process
                // queue) rather than accepting-then-queuing.
                await semaphore.WaitAsync(stoppingToken);

                TcpClient client;
                try
                {
                    client = await listener.AcceptTcpClientAsync(stoppingToken);
                }
                catch (OperationCanceledException)
                {
                    semaphore.Release();
                    break;
                }
                catch (Exception e)
                {
                    logger.LogError(e, "Failed to accept connection");
                    semaphore.Release();
                    continue;
                }

                _ = HandleConnectionWithPermitAsync(client, semaphore, stoppingToken);
            }
        }
        finally
        {
            listener.Stop();
        }
    }

    private async Task HandleConnectionWithPermitAsync(TcpClient client, SemaphoreSlim semaphore, CancellationToken hostStopping)
    {
        try
        {
            await HandleConnectionAsync(client, hostStopping);
        }
        catch (Exception e)
        {
            logger.LogError(e, "Connection handling error");
        }
        finally
        {
            semaphore.Release();
            client.Dispose();
        }
    }

    private async Task HandleConnectionAsync(TcpClient client, CancellationToken hostStopping)
    {
        var remoteEndpoint = client.Client.RemoteEndPoint as IPEndPoint;
        client.NoDelay = true;

        using var timeoutCts = CancellationTokenSource.CreateLinkedTokenSource(hostStopping);
        timeoutCts.CancelAfter(TimeSpan.FromSeconds(options.TimeoutSeconds));

        var stream = client.GetStream();

        string request;
        try
        {
            request = await ReadRequestAsync(stream, timeoutCts.Token);
        }
        catch (OperationCanceledException)
        {
            logger.LogDebug("Request read timeout from {RemoteEndpoint}", remoteEndpoint);
            return;
        }

        if (options.DumpTraffic)
        {
            await DumpToFileAsync(Path.Combine(options.DumpDir, $"query_{DateTimeOffset.UtcNow.ToUnixTimeMilliseconds()}.txt"), request);
        }

        var colorProtocol = new ColorProtocol { Enabled = options.EnableColor };
        var isCapabilityProbe = colorProtocol.ParseHeaders(request);

        if (isCapabilityProbe)
        {
            var capabilityResponse = Encoding.UTF8.GetBytes(colorProtocol.GetCapabilityResponse());
            await stream.WriteAsync(capabilityResponse, hostStopping);
            return;
        }

        var queryLine = request
            .Trim()
            .Split('\n')
            .Select(l => l.TrimEnd('\r'))
            .FirstOrDefault(l => !l.Trim().StartsWith("X-WHOIS-COLOR", StringComparison.OrdinalIgnoreCase))
            ?? string.Empty;

        var query = queryLine.Trim();
        if (query.Length == 0)
        {
            logger.LogDebug("Received empty query from {RemoteEndpoint}", remoteEndpoint);
            return;
        }

        logger.LogDebug("Received query from {RemoteEndpoint}: {Query}", remoteEndpoint, query);

        var queryType = QueryAnalyzer.Analyze(query);
        var colorScheme = colorProtocol.ShouldColorize() ? colorProtocol.Scheme : null;

#if FEATURE_TELEMETRY
        var telemetryStopwatch = Stopwatch.StartNew();
#endif

        string formattedResponse;
        try
        {
            var result = await queryProcessor.ProcessAsync(query, queryType, colorScheme, hostStopping);
            formattedResponse = FormatSuccessResponse(result);
        }
        catch (Exception e)
        {
            logger.LogError(e, "WHOIS query error for {Query}", query);
            formattedResponse = FormatErrorResponse(e, colorProtocol.ShouldColorize());
        }

#if FEATURE_TELEMETRY
        telemetryService.RecordQuery(
            query, queryType.GetType().Name, remoteEndpoint?.Address.ToString() ?? "unknown", telemetryStopwatch.ElapsedMilliseconds);
#endif

        if (options.DumpTraffic)
        {
            await DumpToFileAsync(Path.Combine(options.DumpDir, $"response_{DateTimeOffset.UtcNow.ToUnixTimeMilliseconds()}.txt"), formattedResponse);
        }

        var responseBytes = Encoding.UTF8.GetBytes(formattedResponse);
        await stream.WriteAsync(responseBytes, hostStopping);
        await stream.FlushAsync(hostStopping);

        stats.RecordRequest(responseBytes.Length);

        // RFC 3912: the server, not the client, must initiate connection close.
        client.Client.Shutdown(SocketShutdown.Send);
    }

    private static async Task<string> ReadRequestAsync(NetworkStream stream, CancellationToken ct)
    {
        var buffer = new byte[8192];
        var request = new StringBuilder(512);
        var totalRead = 0;

        while (true)
        {
            var read = await stream.ReadAsync(buffer, ct);
            if (read == 0)
            {
                break;
            }

            request.Append(Encoding.UTF8.GetString(buffer, 0, read));
            totalRead += read;

            if (request.ToString().Contains("\r\n") || totalRead > 900)
            {
                break;
            }
        }

        return request.ToString();
    }

    private static string FormatSuccessResponse(string body)
    {
        var sb = new StringBuilder();
        sb.Append(WhoisServer.Services.Whois.WhoisServerConstants.ServerBanner).Append("\r\n");
        sb.Append("% The objects are in RPSL format\r\n");
        sb.Append("% Please report any issues to noc@akae.re\r\n");
        sb.Append("\r\n");
        sb.Append(body);

        if (!sb.ToString().EndsWith("\r\n", StringComparison.Ordinal))
        {
            sb.Append("\r\n");
        }

        return sb.ToString();
    }

    private static string FormatErrorResponse(Exception e, bool shouldColorize)
    {
        var sb = new StringBuilder();
        sb.Append(WhoisServer.Services.Whois.WhoisServerConstants.ServerBanner).Append("\r\n");
        sb.Append("% Please report any issues to noc@akae.re\r\n");
        sb.Append("\r\n");

        var errorMsg = $"% Error: {e.Message}\r\n";
        var colored = shouldColorize ? $"\x1b[91m{errorMsg}\x1b[0m" : errorMsg;
        sb.Append(colored);
        sb.Append("\r\n");

        return sb.ToString();
    }

    private async Task DumpToFileAsync(string path, string content)
    {
        try
        {
            await File.WriteAllTextAsync(path, content);
        }
        catch (Exception e)
        {
            logger.LogWarning(e, "Failed to dump traffic to {Path}", path);
        }
    }
}

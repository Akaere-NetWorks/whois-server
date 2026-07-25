using System.Net;
using System.Net.Sockets;
using System.Text;
using Microsoft.Extensions.Hosting;
using WhoisServer.Host;
using Xunit;

namespace WhoisServer.Integration.Tests;

/// <summary>
/// End-to-end tests against the real raw TCP WHOIS listener (plan.md's testing strategy §8):
/// boots the exact same DI-wired host Program.cs uses (via <see cref="HostFactory"/>, not a
/// reimplementation of it), on a free port and an isolated temp cache directory, and drives it
/// with a plain <see cref="TcpClient"/> the way a real `whois` client would.
///
/// A couple of these hit real external services (whois.iana.org, RIPE's WHOIS/RDAP servers) since
/// that's the only way to genuinely verify the resolution pipeline works — see plan.md §8. If run
/// somewhere without outbound network access, those specific tests will fail; the framing/protocol
/// tests do not require network access.
/// </summary>
public sealed class WhoisServerIntegrationTests : IAsyncLifetime
{
    private readonly string _cacheDir = Path.Combine(Path.GetTempPath(), $"whois-it-{Guid.NewGuid():N}");
    private IHost? _host;
    private int _port;

    public async Task InitializeAsync()
    {
        _port = GetFreeTcpPort();
        var options = new CliOptions
        {
            Host = "127.0.0.1",
            Port = _port,
            WebPort = GetFreeTcpPort(),
            TimeoutSeconds = 20,
        };
        _host = HostFactory.Build(options, _cacheDir);
        await HostFactory.LoadPatchesAsync(_host);
        await _host.StartAsync();

        // Give the BackgroundService a moment to actually start listening.
        await WaitUntilPortIsListeningAsync(_port);
    }

    public async Task DisposeAsync()
    {
        if (_host is not null)
        {
            await _host.StopAsync();
            _host.Dispose();
        }

        try
        {
            Directory.Delete(_cacheDir, recursive: true);
        }
        catch
        {
            // Best-effort cleanup.
        }
    }

    private static int GetFreeTcpPort()
    {
        var listener = new TcpListener(IPAddress.Loopback, 0);
        listener.Start();
        var port = ((IPEndPoint)listener.LocalEndpoint).Port;
        listener.Stop();
        return port;
    }

    private static async Task WaitUntilPortIsListeningAsync(int port)
    {
        for (var i = 0; i < 50; i++)
        {
            try
            {
                using var probe = new TcpClient();
                await probe.ConnectAsync(IPAddress.Loopback, port);
                return;
            }
            catch (SocketException)
            {
                await Task.Delay(100);
            }
        }

        throw new TimeoutException($"Server never started listening on port {port}");
    }

    private async Task<string> SendRawQueryAsync(string query)
    {
        using var client = new TcpClient();
        await client.ConnectAsync(IPAddress.Loopback, _port);
        await using var stream = client.GetStream();

        await stream.WriteAsync(Encoding.UTF8.GetBytes(query + "\r\n"));

        var buffer = new byte[16384];
        var response = new StringBuilder();
        while (true)
        {
            int read;
            try
            {
                read = await stream.ReadAsync(buffer);
            }
            catch (IOException)
            {
                break; // Server closed the connection (expected per RFC 3912).
            }

            if (read == 0)
            {
                break;
            }

            response.Append(Encoding.UTF8.GetString(buffer, 0, read));
        }

        return response.ToString();
    }

    [Fact]
    public async Task HelpQuery_ReturnsBannerAndHelpText()
    {
        var response = await SendRawQueryAsync("HELP");

        Assert.StartsWith("% Akaere NetWorks Whois Server\r\n", response);
        Assert.Contains("Help Information", response);
        Assert.EndsWith("\r\n", response);
    }

    [Fact]
    public async Task ColorCapabilityProbe_ReturnsCapabilityResponse_NotAWhoisAnswer()
    {
        var response = await SendRawQueryAsync("X-WHOIS-COLOR-PROBE: 1\r\nHELP");

        Assert.Equal("X-WHOIS-COLOR-SUPPORT: 1.0 schemes=ripe,ripe-dark,bgptools,bgptools-dark\r\n\r\n", response);
    }

    [Fact]
    public async Task EmptyQuery_ClosesConnectionWithNoResponse()
    {
        var response = await SendRawQueryAsync("");
        Assert.Equal(string.Empty, response);
    }

    [Fact]
    public async Task ColorizedQuery_ProducesAnsiEscapeCodes()
    {
        var response = await SendRawQueryAsync("X-WHOIS-COLOR: ripe\r\nHELP");
        Assert.Contains("\x1b[", response);
    }

    [Fact]
    public async Task DomainQuery_ResolvesViaRealRdapOrWhois()
    {
        var response = await SendRawQueryAsync("example.com");

        Assert.StartsWith("% Akaere NetWorks Whois Server\r\n", response);
        // Either the RDAP path or the WHOIS-fallback path should have produced *something*
        // beyond just the banner — this is the actual resolution pipeline hitting real registries.
        Assert.True(response.Length > 300, $"Expected a substantive response, got {response.Length} chars:\n{response}");
    }

    [Fact]
    public async Task DescSuffix_ReturnsSummaryEnvelopeEvenWhenNoDescrFieldsFound()
    {
        var response = await SendRawQueryAsync("AS213605-DESC");

        Assert.Contains("Description Query Results for: AS213605", response);
    }
}

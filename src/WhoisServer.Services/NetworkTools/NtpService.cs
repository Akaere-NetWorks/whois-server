using System.Net.Sockets;
using System.Text;

namespace WhoisServer.Services.NetworkTools;

/// <summary>Ported from src/services/ntp.rs — the <c>-NTP</c> suffix: a raw NTPv3 client-mode
/// query over UDP/123, reporting stratum/offset/round-trip without touching the system clock.</summary>
public sealed class NtpService
{
    private const long NtpEpochOffsetSeconds = 2208988800L; // seconds between 1900 and 1970

    public async Task<string> ProcessAsync(string server, CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(server))
        {
            return """
                % NTP Time Synchronization Test
                % Error: No server specified
                %
                % Usage: <server>-NTP
                %
                % Examples:
                %   pool.ntp.org-NTP
                %   time.google.com-NTP
                %   time.cloudflare.com-NTP

                """;
        }

        try
        {
            return await QueryNtpServerAsync(server, ct);
        }
        catch (Exception e)
        {
            return $"""
                % NTP Time Synchronization Test
                % Server: {server}
                % Error: {e.Message}
                %
                % Possible reasons:
                % - Server is unreachable
                % - Firewall blocking UDP port 123
                % - Invalid server address
                % - Server is not responding
                %
                % Try these public NTP servers:
                %   pool.ntp.org
                %   time.google.com
                %   time.cloudflare.com

                """;
        }
    }

    private static async Task<string> QueryNtpServerAsync(string server, CancellationToken ct)
    {
        using var udp = new UdpClient();
        udp.Client.ReceiveTimeout = 5000;
        udp.Client.SendTimeout = 5000;

        using var timeoutCts = CancellationTokenSource.CreateLinkedTokenSource(ct);
        timeoutCts.CancelAfter(TimeSpan.FromSeconds(5));

        udp.Connect(server, 123);

        var request = new byte[48];
        request[0] = 0x1B; // LI=0, VN=3, Mode=3 (client)

        var t1 = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds() * 1000;
        await udp.SendAsync(request, timeoutCts.Token);

        var result = await udp.ReceiveAsync(timeoutCts.Token);
        var t4 = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds() * 1000;

        if (result.Buffer.Length < 48)
        {
            throw new InvalidOperationException("Invalid NTP response size");
        }

        var response = result.Buffer;
        var stratum = response[1];
        var precision = (sbyte)response[3];
        var rootDelay = ReadUInt32BigEndian(response, 4);
        var rootDispersion = ReadUInt32BigEndian(response, 8);
        var rxTimestamp = ReadUInt64BigEndian(response, 32);
        var txTimestamp = ReadUInt64BigEndian(response, 40);

        var t2 = NtpToUnixMicros(rxTimestamp);
        var t3 = NtpToUnixMicros(txTimestamp);

        var offsetMicros = ((t2 - t1) + (t3 - t4)) / 2;
        var delayMicros = (t4 - t1) - (t3 - t2);
        var offsetMs = offsetMicros / 1000.0;
        var delayMs = delayMicros / 1000.0;

        var stratumDesc = stratum switch
        {
            0 => "Unspecified or invalid",
            1 => "Primary reference (e.g., GPS, atomic clock)",
            >= 2 and <= 15 => "Secondary reference (via NTP)",
            _ => "Reserved",
        };

        var sb = new StringBuilder();
        sb.Append("% NTP Time Synchronization Test\n");
        sb.Append($"% Server: {server}\n%\n");
        sb.Append("% Server Information:\n");
        sb.Append($"stratum:         {stratum} ({stratumDesc})\n");
        sb.Append($"precision:       2^{precision} seconds\n");
        sb.Append($"root-delay:      {(uint)(rootDelay / 65536.0 * 1000.0)} ms\n");
        sb.Append($"root-dispersion: {(uint)(rootDispersion / 65536.0 * 1000.0)} ms\n%\n");
        sb.Append("% Time Information:\n");
        sb.Append($"server-time:     {FormatTimestamp(t3 / 1_000_000)}\n");
        sb.Append($"local-time:      {FormatTimestamp(t4 / 1_000_000)}\n%\n");
        sb.Append("% Synchronization Metrics:\n");
        sb.Append($"offset:          {offsetMs:F3} ms ({offsetMs / 1000.0:F6} seconds)\n");
        sb.Append($"round-trip:      {delayMs:F3} ms\n%\n");

        var absOffset = Math.Abs(offsetMs);
        sb.Append(absOffset switch
        {
            > 1000.0 => $"% ⚠ WARNING: Clock offset is {offsetMs / 1000.0:F3} seconds\n",
            > 100.0 => $"% ⚠ Clock offset is significant: {offsetMs:F1}ms\n",
            > 10.0 => $"% ✓ Clock is synchronized (offset: {offsetMs:F1}ms)\n",
            _ => $"% ✓ Excellent synchronization! (offset: {offsetMs:F2}ms)\n",
        });

        sb.Append("%\n% Note: This is a test query only. System time was not modified.\n");
        return sb.ToString();
    }

    private static uint ReadUInt32BigEndian(byte[] buffer, int offset) =>
        (uint)((buffer[offset] << 24) | (buffer[offset + 1] << 16) | (buffer[offset + 2] << 8) | buffer[offset + 3]);

    private static ulong ReadUInt64BigEndian(byte[] buffer, int offset)
    {
        ulong value = 0;
        for (var i = 0; i < 8; i++)
        {
            value = (value << 8) | buffer[offset + i];
        }

        return value;
    }

    private static long NtpToUnixMicros(ulong ntpTimestamp)
    {
        var seconds = (long)(ntpTimestamp >> 32);
        var fraction = (double)(ntpTimestamp & 0xFFFFFFFF);
        var micros = (long)(fraction / 4294967296.0 * 1_000_000.0);
        return (seconds - NtpEpochOffsetSeconds) * 1_000_000 + micros;
    }

    private static string FormatTimestamp(long unixSeconds) =>
        DateTimeOffset.FromUnixTimeSeconds(unixSeconds).UtcDateTime.ToString("yyyy-MM-dd HH:mm:ss") + " UTC";
}

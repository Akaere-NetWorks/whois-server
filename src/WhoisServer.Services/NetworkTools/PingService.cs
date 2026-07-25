using System.Text;

namespace WhoisServer.Services.NetworkTools;

/// <summary>Ported from src/services/ping.rs — the <c>-PING</c> suffix, using the public
/// Globalping API (globalping.io) to ping from real-world vantage points. Supports an optional
/// trailing location code: <c>target-location</c> (e.g. "1.1.1.1-tw").</summary>
public sealed class PingService(GlobalpingClient globalping, IpInfoClient ipInfo, DohClient dohClient)
{
    public async Task<string> ProcessAsync(string query, CancellationToken ct = default)
    {
        var (target, location) = LocationQueryParser.Parse(query);

        try
        {
            var request = new GlobalpingRequest(
                MeasurementType: "ping",
                Target: target,
                Limit: 5,
                MeasurementOptions: new PingOptions(Packets: 4, Protocol: "ICMP"),
                Locations: location is null ? null : [new MeasurementLocation(location)],
                InProgressUpdates: false);

            var id = await globalping.SubmitMeasurementAsync(request, ct);
            var results = await globalping.WaitForResultsAsync(id, 30, ct);
            return await FormatAsync(results, target, ct);
        }
        catch (Exception e)
        {
            return $"Ping failed: {e.Message}\n";
        }
    }

    private async Task<string> FormatAsync(GlobalpingResult results, string target, CancellationToken ct)
    {
        if (results.Results.Count == 0)
        {
            return $"No results received for ping to {target}\n";
        }

        var sb = new StringBuilder();
        foreach (var probeResult in results.Results)
        {
            var testResult = probeResult.Result;
            var probe = probeResult.Probe;
            var targetIp = testResult.ResolvedAddress ?? target;

            var info = await ipInfo.GetIpInfoAsync(targetIp, ct);
            var ptrRecords = await dohClient.QueryPtrAsync(targetIp, ct);

            sb.Append($"PING {target} ({probe.Network})");
            if (info is not null)
            {
                sb.Append($" from {info.Country}");
            }

            sb.Append('\n');
            sb.Append($"Probe: {probe.Network} - {probe.City ?? "Unknown"}, {probe.State ?? ""}, {probe.Country}\n");

            if (testResult.Stats is { } stats)
            {
                var lossRate = stats.Total > 0 ? (int)((double)stats.Loss / stats.Total * 100) : 0;
                sb.Append($"{stats.Total} packets transmitted, {stats.Received} received, {lossRate}% packet loss\n");
                sb.Append($"rtt min/avg/max = {stats.Min:F2}/{stats.Avg:F2}/{stats.Max:F2} ms\n");
            }

            if (info is not null)
            {
                sb.Append($"  ASN: {info.Asn} | {info.Country} | {info.Continent} | {info.AsDomain}\n");
            }

            if (ptrRecords.Count > 0)
            {
                sb.Append($"  PTR: {string.Join(", ", ptrRecords)}\n");
            }

            if (testResult.Timings is { Count: > 0 } timings)
            {
                sb.Append("  Times: ").Append(string.Join(", ", timings.Select(t => $"{t.Rtt:F2} ms"))).Append('\n');
            }

            sb.Append('\n');
        }

        return sb.ToString();
    }
}

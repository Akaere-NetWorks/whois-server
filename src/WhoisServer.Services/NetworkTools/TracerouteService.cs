using System.Text;

namespace WhoisServer.Services.NetworkTools;

/// <summary>Ported from src/services/traceroute.rs — the <c>-TRACE</c>/<c>-TRACEROUTE</c> suffix,
/// using the public Globalping API for hop-by-hop path analysis.</summary>
public sealed class TracerouteService(GlobalpingClient globalping, IpInfoClient ipInfo, DohClient dohClient)
{
    public async Task<string> ProcessAsync(string query, CancellationToken ct = default)
    {
        var (target, location) = LocationQueryParser.Parse(query);

        try
        {
            var request = new GlobalpingRequest(
                MeasurementType: "traceroute",
                Target: target,
                Limit: 1,
                MeasurementOptions: new TracerouteOptions(Protocol: "ICMP"),
                Locations: location is null ? null : [new MeasurementLocation(location)],
                InProgressUpdates: false);

            var id = await globalping.SubmitMeasurementAsync(request, ct);
            var results = await globalping.WaitForResultsAsync(id, 60, ct);
            return await FormatAsync(results, target, ct);
        }
        catch (Exception e)
        {
            return $"Traceroute measurement timed out or failed: {e.Message}\n";
        }
    }

    private async Task<string> FormatAsync(GlobalpingResult results, string target, CancellationToken ct)
    {
        if (results.Results.Count == 0)
        {
            return $"No results received for traceroute to {target}\n";
        }

        var sb = new StringBuilder();
        foreach (var probeResult in results.Results)
        {
            var testResult = probeResult.Result;
            var probe = probeResult.Probe;
            var targetIp = testResult.ResolvedAddress ?? target;

            sb.Append($"traceroute to {targetIp}, 30 hops max, 52 bytes payload, ICMP mode\n");
            sb.Append($"Probe: {probe.Network} - {probe.City ?? "Unknown"}, {probe.Country}\n\n");

            if (testResult.Hops is not { Count: > 0 } hops)
            {
                sb.Append("No hops data available in traceroute results\n");
                sb.Append('\n');
                continue;
            }

            for (var hopNum = 0; hopNum < hops.Count; hopNum++)
            {
                var hop = hops[hopNum];
                if (hop.ResolvedAddress is not { } resolvedAddress)
                {
                    sb.Append($"{hopNum + 1,3}   *\n");
                    continue;
                }

                var info = await ipInfo.GetIpInfoAsync(resolvedAddress, ct);
                var ptrRecords = await dohClient.QueryPtrAsync(resolvedAddress, ct);

                sb.Append($"{hopNum + 1,3}   {resolvedAddress,-15}");
                sb.Append(info is not null
                    ? $"   {info.Asn,-15}  {info.AsName,-20}  {info.CountryCode,-6}  {info.ContinentCode,-10}  {info.AsDomain}\n"
                    : "   *             *                      *           *\n");

                if (ptrRecords.Count > 0)
                {
                    sb.Append($"      {ptrRecords[0],-15}\n");
                }

                if (hop.Timings is { Count: > 0 } timings)
                {
                    sb.Append(new string(' ', 48)).Append(string.Join(" / ", timings.Select(t => $"{t.Rtt:F2} ms"))).Append('\n');
                }
                else
                {
                    sb.Append(new string(' ', 48)).Append("*\n");
                }
            }

            sb.Append('\n');
        }

        return sb.ToString();
    }
}

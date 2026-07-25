using System.Net;
using System.Text;

namespace WhoisServer.Services.NetworkTools;

/// <summary>Ported from src/services/dns.rs — the <c>-DNS</c> suffix: forward lookups for a
/// domain (A/AAAA/CNAME/MX/TXT/NS/SOA via Cloudflare DoH) or a reverse (PTR) lookup for an IP.</summary>
public sealed class DnsService(DohClient dohClient)
{
    public async Task<string> ProcessAsync(string baseQuery, CancellationToken ct = default)
    {
        if (IPAddress.TryParse(baseQuery, out var ip))
        {
            return await QueryReverseDnsAsync(ip, ct);
        }

        if (IsDomainName(baseQuery))
        {
            return await QueryForwardDnsAsync(baseQuery, ct);
        }

        return $"Invalid DNS query format. Please provide a valid domain name or IP address.\nQuery: {baseQuery}\n";
    }

    private async Task<string> QueryForwardDnsAsync(string domain, CancellationToken ct)
    {
        var results = await dohClient.QueryBatchAsync(domain, DohClient.StandardRecordTypes, ct);
        if (results.Count == 0)
        {
            return $"No DNS records found for domain: {domain}\n";
        }

        var sb = new StringBuilder();
        sb.Append($"DNS Records for {domain}:\n");
        foreach (var type in DohClient.StandardRecordTypes)
        {
            if (results.TryGetValue(type, out var answers) && answers.Count > 0)
            {
                sb.Append($"\n{type} Records:\n");
                foreach (var answer in answers)
                {
                    sb.Append(FormatAnswer(answer, type));
                }
            }
        }

        return sb.ToString();
    }

    private async Task<string> QueryReverseDnsAsync(IPAddress ip, CancellationToken ct)
    {
        var records = await dohClient.QueryPtrAsync(ip.ToString(), ct);
        if (records.Count == 0)
        {
            return $"No reverse DNS record found for IP: {ip}\n";
        }

        var sb = new StringBuilder();
        sb.Append($"Reverse DNS Results for {ip}:\n\nPTR Records:\n");
        foreach (var record in records)
        {
            sb.Append($"  {record}\n");
        }

        return sb.ToString();
    }

    private static string FormatAnswer(DnsAnswer answer, string recordType)
    {
        if (recordType == "MX")
        {
            var spaceIndex = answer.Data.IndexOf(' ');
            if (spaceIndex >= 0)
            {
                return $"  {answer.Data[..spaceIndex]} {answer.Data[(spaceIndex + 1)..]} (TTL: {answer.Ttl})\n";
            }
        }

        if (recordType == "TXT")
        {
            var data = answer.Data.Length >= 2 && answer.Data.StartsWith('"') && answer.Data.EndsWith('"')
                ? answer.Data[1..^1]
                : answer.Data;
            return $"  \"{data}\" (TTL: {answer.Ttl})\n";
        }

        return $"  {answer.Data} (TTL: {answer.Ttl})\n";
    }

    public static bool IsDomainName(string query)
    {
        if (query.Length == 0 || query.Length > 253 || !query.Contains('.'))
        {
            return false;
        }

        if (IPAddress.TryParse(query, out _))
        {
            return false;
        }

        var parts = query.Split('.');
        if (parts.Length < 2)
        {
            return false;
        }

        foreach (var part in parts)
        {
            if (part.Length == 0 || part.Length > 63)
            {
                return false;
            }

            if (!part.All(c => char.IsAsciiLetterOrDigit(c) || c == '-'))
            {
                return false;
            }

            if (part.StartsWith('-') || part.EndsWith('-'))
            {
                return false;
            }
        }

        return true;
    }
}

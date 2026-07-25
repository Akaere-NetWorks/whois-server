using System.Net;
using System.Net.Http.Json;
using System.Text.Json.Serialization;

namespace WhoisServer.Services.NetworkTools;

/// <summary>Ported from src/services/utils/doh.rs — Cloudflare DNS-over-HTTPS client.</summary>
public sealed class DohClient(HttpClient httpClient)
{
    private const string CloudflareDohUrl = "https://cloudflare-dns.com/dns-query";

    public static readonly string[] StandardRecordTypes = ["A", "AAAA", "CNAME", "MX", "TXT", "NS", "SOA"];

    public async Task<DnsResponse> QueryAsync(string name, string recordType, CancellationToken ct = default)
    {
        using var request = new HttpRequestMessage(HttpMethod.Get,
            $"{CloudflareDohUrl}?name={Uri.EscapeDataString(name)}&type={recordType}&do=false");
        request.Headers.TryAddWithoutValidation("Accept", "application/dns-json");

        using var response = await httpClient.SendAsync(request, ct);
        response.EnsureSuccessStatusCode();

        return await response.Content.ReadFromJsonAsync<DnsResponse>(cancellationToken: ct)
            ?? throw new InvalidOperationException("Empty DOH response");
    }

    public async Task<Dictionary<string, List<DnsAnswer>>> QueryBatchAsync(
        string name, IEnumerable<string> recordTypes, CancellationToken ct = default)
    {
        var tasks = recordTypes.Select(async (string type) =>
        {
            try
            {
                DnsResponse? response = await QueryAsync(name, type, ct);
                return (Type: type, Response: response);
            }
            catch
            {
                return (Type: type, Response: (DnsResponse?)null);
            }
        });

        var results = await Task.WhenAll(tasks);
        var output = new Dictionary<string, List<DnsAnswer>>();
        foreach (var (type, response) in results)
        {
            if (response is { Status: 0, Answer.Count: > 0 })
            {
                output[type] = response.Answer;
            }
        }

        return output;
    }

    /// <summary>Returns hostnames from PTR records for an IP, or an empty list on any failure
    /// (matching the Rust version's "returns empty Vec, not Err" convenience behavior).</summary>
    public async Task<List<string>> QueryPtrAsync(string ip, CancellationToken ct = default)
    {
        if (!IPAddress.TryParse(ip, out var address))
        {
            return [];
        }

        var ptrName = CreatePtrName(address);

        try
        {
            var response = await QueryAsync(ptrName, "PTR", ct);
            if (response.Status != 0)
            {
                return [];
            }

            return response.Answer
                .Where(a => a.Type == 12)
                .Select(a => a.Data.EndsWith('.') ? a.Data[..^1] : a.Data)
                .ToList();
        }
        catch
        {
            return [];
        }
    }

    public static string CreatePtrName(IPAddress ip)
    {
        if (ip.AddressFamily == System.Net.Sockets.AddressFamily.InterNetwork)
        {
            var octets = ip.GetAddressBytes();
            return $"{octets[3]}.{octets[2]}.{octets[1]}.{octets[0]}.in-addr.arpa";
        }

        var bytes = ip.GetAddressBytes();
        var nibbles = new List<string>();
        for (var i = bytes.Length - 1; i >= 0; i--)
        {
            nibbles.Add((bytes[i] & 0x0f).ToString("x"));
            nibbles.Add(((bytes[i] & 0xf0) >> 4).ToString("x"));
        }

        return $"{string.Join('.', nibbles)}.ip6.arpa";
    }
}

public sealed record DnsResponse(
    [property: JsonPropertyName("Status")] int Status,
    [property: JsonPropertyName("Answer")] List<DnsAnswer>? AnswerOrNull)
{
    [JsonIgnore]
    public List<DnsAnswer> Answer => AnswerOrNull ?? [];
}

public sealed record DnsAnswer(
    [property: JsonPropertyName("name")] string Name,
    [property: JsonPropertyName("type")] int Type,
    [property: JsonPropertyName("data")] string Data,
    [property: JsonPropertyName("TTL")] int Ttl);

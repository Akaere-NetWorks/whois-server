using WhoisServer.Services.Whois;

namespace WhoisServer.Services.Bgp;

/// <summary>
/// Direct WHOIS queries to one of the 12 IRR registries (e.g. <c>-RADB</c>, <c>-ARIN</c>). Ported
/// from what was 12 near-identical Rust query functions into one generic, parameterized service —
/// see plan.md §3 (the same collapsing done for the package-manager services).
/// </summary>
public sealed class IrrDirectService(WhoisClient whoisClient)
{
    public async Task<string> ProcessAsync(string registryKey, string baseQuery, CancellationToken ct = default)
    {
        if (!IrrRegistryConstants.Registries.TryGetValue(registryKey, out var registry))
        {
            return $"% IRR Query Error\n% Unknown registry: {registryKey}\n";
        }

        try
        {
            var response = await whoisClient.QueryAsync(baseQuery, registry.Server, registry.Port,
                WhoisServerConstants.DefaultTimeout, ct);
            return $"% {registryKey} IRR Query\n% Data from {registry.Server}\n\n{response}";
        }
        catch (Exception e)
        {
            return $"% {registryKey} IRR Query Failed\n% Query: {baseQuery}\n% Error: {e.Message}\n";
        }
    }
}

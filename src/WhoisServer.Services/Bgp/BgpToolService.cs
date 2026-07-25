using WhoisServer.Services.Whois;

namespace WhoisServer.Services.Bgp;

/// <summary>Ported from src/services/bgptool.rs — queries bgp.tools' own WHOIS server (it
/// understands a " -v {query}" verbose flag) for the <c>-BGPTOOL</c> suffix.</summary>
public sealed class BgpToolService(WhoisClient whoisClient)
{
    private const string BgpToolsServer = "bgp.tools";

    public async Task<string> ProcessAsync(string baseQuery, CancellationToken ct = default)
    {
        try
        {
            var response = await whoisClient.QueryAsync($" -v {baseQuery}", BgpToolsServer,
                WhoisServerConstants.DefaultWhoisPort, WhoisServerConstants.DefaultTimeout, ct);
            return $"% BGP Tools Query\n% Data from bgp.tools\n\n{response}";
        }
        catch (Exception e)
        {
            return $"% BGP Tools Query Failed\n% Query: {baseQuery}\n% Error: {e.Message}\n";
        }
    }
}

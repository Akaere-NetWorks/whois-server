namespace WhoisServer.Plugins;

/// <summary>
/// Rejects any request whose host isn't in <paramref name="allowedDomains"/> — the
/// <see cref="DelegatingHandler"/> equivalent of the Rust sandbox's <c>http_get</c> whitelist
/// check, enforced on the plugin's injected <see cref="HttpClient"/> itself rather than inside a
/// Lua-callable wrapper function (there's no VM boundary here to hang the check off of instead).
/// An empty list means no restriction (matches the Rust version's <c>allowed_domains = []</c>
/// meaning "no whitelist enforced" — a plugin manifest that wants network access without a
/// whitelist just omits <c>allowedDomains</c>).
/// </summary>
public sealed class DomainWhitelistHttpHandler(IReadOnlyList<string> allowedDomains) : DelegatingHandler(new HttpClientHandler())
{
    private readonly HashSet<string> _allowed = new(allowedDomains, StringComparer.OrdinalIgnoreCase);

    protected override Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken cancellationToken)
    {
        if (_allowed.Count > 0 && (request.RequestUri is null || !_allowed.Contains(request.RequestUri.Host)))
        {
            throw new InvalidOperationException(
                $"Domain '{request.RequestUri?.Host}' is not in this plugin's allowed-domains whitelist");
        }

        return base.SendAsync(request, cancellationToken);
    }
}

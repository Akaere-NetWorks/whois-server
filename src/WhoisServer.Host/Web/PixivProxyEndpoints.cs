namespace WhoisServer.Host.Web;

/// <summary>
/// Ported from src/web/pixiv_proxy.rs. Proxies <c>i.pximg.net</c> image requests with a spoofed
/// User-Agent/Referer to bypass Pixiv's hotlink protection — the same trick every third-party
/// Pixiv viewer uses. <see cref="WhoisServer.Services.Pixiv.PixivClient"/> rewrites artwork image
/// URLs to point here instead of directly at pximg.net.
/// </summary>
public static class PixivProxyEndpoints
{
    public static void MapPixivProxy(this WebApplication app)
    {
        app.MapGet("/pixiv-proxy/{**path}", async (string path, IHttpClientFactory httpClientFactory, HttpContext ctx, CancellationToken ct) =>
        {
            var client = httpClientFactory.CreateClient();
            client.Timeout = TimeSpan.FromSeconds(30);

            using var request = new HttpRequestMessage(HttpMethod.Get, $"https://i.pximg.net/{path}");
            request.Headers.TryAddWithoutValidation("User-Agent", "TelegramBot (like TwitterBot)");
            request.Headers.TryAddWithoutValidation("Referer", "https://www.pixiv.net/");

            HttpResponseMessage response;
            try
            {
                response = await client.SendAsync(request, ct);
            }
            catch (HttpRequestException)
            {
                return Results.StatusCode(StatusCodes.Status502BadGateway);
            }

            using (response)
            {
                if (!response.IsSuccessStatusCode)
                {
                    return Results.StatusCode((int)response.StatusCode);
                }

                var contentType = response.Content.Headers.ContentType?.ToString() ?? "application/octet-stream";
                var bytes = await response.Content.ReadAsByteArrayAsync(ct);

                ctx.Response.Headers.CacheControl = "public, max-age=86400";
                ctx.Response.Headers.AccessControlAllowOrigin = "*";
                return Results.Bytes(bytes, contentType);
            }
        }).WithName("PixivProxy");

        app.MapGet("/pixiv-proxy-health", () => Results.Text("Pixiv Proxy OK")).WithName("PixivProxyHealth");
    }
}

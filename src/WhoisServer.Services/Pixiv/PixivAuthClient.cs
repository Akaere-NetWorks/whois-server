using System.Net.Http.Json;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json.Serialization;

namespace WhoisServer.Services.Pixiv;

public sealed class PixivAuthException(string message) : Exception(message);

/// <summary>
/// Ported from src/services/pixiv/auth.rs. Pixiv's app API requires OAuth2 refresh-token
/// authentication plus a per-request <c>X-Client-Time</c>/<c>X-Client-Hash</c> pair (MD5 of
/// client-time + a fixed hash secret, base64-encoded) — both the client ID/secret and hash secret
/// below are the long-public "Pixiv Android app" credentials used openly across the Pixiv API
/// client ecosystem for years (pixivpy, etc.), already hardcoded in the Rust source being ported;
/// not a new secret exposure. The actual user-specific credential is <c>PIXIV_REFRESH_TOKEN</c>,
/// read from the environment (see <see cref="WhoisServer.Host.HostFactory"/>), never hardcoded.
/// </summary>
public sealed class PixivAuthClient(HttpClient httpClient, string? refreshToken) : IDisposable
{
    private const string ClientId = "MOBrBDS8blbauoSck0ZfDbtuzpyT";
    private const string ClientSecret = "lsACyCD94FhDUtGTXi3QzcFE2uU1hqtDaKeqrdwj";
    private const string HashSecret = "28c1fdd170a5204386cb1313c7077b34f83e4aaf4aa829ce78c231e05b0bae2c";
    private const string AuthUrl = "https://oauth.secure.pixiv.net/auth/token";
    public const string UserAgent = "PixivAndroidApp/5.0.64 (Android 6.0; Pixiv)";

    private readonly SemaphoreSlim _lock = new(1, 1);
    private string? _accessToken;
    private DateTimeOffset _expiresAt;

    public bool IsConfigured => !string.IsNullOrEmpty(refreshToken);

    public async Task<string> GetAccessTokenAsync(CancellationToken ct = default)
    {
        if (!IsConfigured)
        {
            throw new PixivAuthException("Pixiv is not configured: set the PIXIV_REFRESH_TOKEN environment variable");
        }

        await _lock.WaitAsync(ct);
        try
        {
            if (_accessToken is not null && DateTimeOffset.UtcNow + TimeSpan.FromMinutes(5) < _expiresAt)
            {
                return _accessToken;
            }

            using var request = new HttpRequestMessage(HttpMethod.Post, AuthUrl)
            {
                Content = new FormUrlEncodedContent(new Dictionary<string, string>
                {
                    ["client_id"] = ClientId,
                    ["client_secret"] = ClientSecret,
                    ["grant_type"] = "refresh_token",
                    ["refresh_token"] = refreshToken!,
                    ["include_policy"] = "true",
                }),
            };
            request.Headers.TryAddWithoutValidation("User-Agent", UserAgent);

            using var response = await httpClient.SendAsync(request, ct);
            if (!response.IsSuccessStatusCode)
            {
                var body = await response.Content.ReadAsStringAsync(ct);
                throw new PixivAuthException($"Pixiv authentication failed: {body}");
            }

            var auth = await response.Content.ReadFromJsonAsync<AuthResponse>(cancellationToken: ct)
                ?? throw new PixivAuthException("Empty Pixiv auth response");

            _accessToken = auth.AccessToken;
            _expiresAt = DateTimeOffset.UtcNow + TimeSpan.FromSeconds(auth.ExpiresIn);
            return _accessToken;
        }
        finally
        {
            _lock.Release();
        }
    }

    public static string ComputeClientHash(string clientTime)
    {
        // MD5 is mandated by Pixiv's own undocumented app-API protocol, not a security choice we
        // control here — the client hash algorithm can't be swapped without breaking authentication.
#pragma warning disable CA5351
        var hash = MD5.HashData(Encoding.UTF8.GetBytes(clientTime + HashSecret));
#pragma warning restore CA5351
        return Convert.ToBase64String(hash);
    }

    public void Dispose() => _lock.Dispose();

    private sealed record AuthResponse(
        [property: JsonPropertyName("access_token")] string AccessToken,
        [property: JsonPropertyName("refresh_token")] string RefreshToken,
        [property: JsonPropertyName("expires_in")] long ExpiresIn);
}

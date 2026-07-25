using System.Net.Http.Headers;
using System.Net.Http.Json;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace WhoisServer.Services.Packages;

/// <summary>
/// Ported from src/services/packages/npm.rs — the <c>-NPM</c> suffix. Queries
/// <c>https://registry.npmjs.org/&lt;name&gt;</c> (scoped packages are percent-encoded as
/// <c>%40scope%2Fname</c> for the registry call, but kept as <c>@scope/name</c> for the npmjs.com
/// web URL — this asymmetry is intentional and mirrors the Rust version, not a bug). No caching:
/// the registry API is fast and free, same as the Rust version (which never cached this either).
/// </summary>
public sealed class NpmPackageService(HttpClient httpClient)
{
    private const string RegistryUrl = "https://registry.npmjs.org/";

    public async Task<string> ProcessAsync(string packageName, CancellationToken ct = default)
    {
        if (!IsValidPackageName(packageName))
        {
            return FormatNotFound(packageName);
        }

        try
        {
            var package = await QueryAsync(packageName, ct);
            return package is null ? FormatNotFound(packageName) : FormatResponse(package, packageName);
        }
        catch
        {
            return FormatNotFound(packageName);
        }
    }

    /// <summary>Ported from the Rust version's validation block in <c>process_npm_query</c>:
    /// max 214 chars, no spaces, must already be lowercase, and scoped (<c>@scope/name</c>)
    /// packages have their own sub-rules (both halves non-empty, neither starting with
    /// <c>.</c> or <c>_</c>); unscoped names additionally can't start with <c>.</c> or
    /// <c>_</c> either.</summary>
    private static bool IsValidPackageName(string packageName)
    {
        if (packageName.Length == 0 || packageName.Length > 214)
        {
            return false;
        }

        if (packageName.Contains(' ') || packageName.ToLowerInvariant() != packageName)
        {
            return false;
        }

        if (packageName.StartsWith('@'))
        {
            var slashIndex = packageName.IndexOf('/');
            if (slashIndex < 0)
            {
                return false;
            }

            var scope = packageName[1..slashIndex];
            var name = packageName[(slashIndex + 1)..];

            if (scope.Length == 0 || name.Length == 0)
            {
                return false;
            }

            if (scope.StartsWith('.') || scope.StartsWith('_') || name.StartsWith('.') || name.StartsWith('_'))
            {
                return false;
            }
        }
        else if (packageName.StartsWith('.') || packageName.StartsWith('_'))
        {
            return false;
        }

        return true;
    }

    private static string EncodeForRegistry(string packageName) =>
        packageName.StartsWith('@')
            ? packageName.Replace("@", "%40").Replace("/", "%2F")
            : Uri.EscapeDataString(packageName);

    private async Task<NpmPackage?> QueryAsync(string packageName, CancellationToken ct)
    {
        using var request = new HttpRequestMessage(HttpMethod.Get, RegistryUrl + EncodeForRegistry(packageName));
        request.Headers.UserAgent.ParseAdd("Mozilla/5.0 (compatible; WHOIS-Server/1.0)");

        using var response = await httpClient.SendAsync(request, ct);
        if (!response.IsSuccessStatusCode)
        {
            return null;
        }

        var doc = await response.Content.ReadFromJsonAsync<NpmRegistryResponse>(cancellationToken: ct);
        if (doc is null)
        {
            return null;
        }

        var latestVersion = doc.DistTags?.GetValueOrDefault("latest") ?? "unknown";
        var versionData = doc.Versions?.GetValueOrDefault(latestVersion);

        return new NpmPackage(
            doc.Name,
            latestVersion,
            doc.Description ?? versionData?.Description,
            doc.Homepage ?? versionData?.Homepage,
            doc.Repository ?? versionData?.Repository,
            doc.Author ?? versionData?.Author,
            doc.Maintainers,
            doc.License ?? versionData?.License,
            doc.Keywords ?? versionData?.Keywords,
            versionData?.Dependencies,
            versionData?.Dist,
            doc.DistTags);
    }

    private static string FormatResponse(NpmPackage package, string query)
    {
        var sb = new StringBuilder();
        sb.Append($"NPM Package Information: {query}\n");
        sb.Append(new string('=', 60)).Append('\n');

        sb.Append($"package-name: {package.Name}\n");
        sb.Append($"version: {package.Version}\n");

        if (package.Description is { } description)
        {
            sb.Append($"description: {description}\n");
        }

        if (package.Author is { } author)
        {
            sb.Append($"author: {FormatAuthor(author)}\n");
        }

        if (package.License is { } license)
        {
            sb.Append($"license: {license}\n");
        }

        if (package.Homepage is { } homepage)
        {
            sb.Append($"homepage: {homepage}\n");
        }

        var repoUrl = FormatRepository(package.Repository);
        if (repoUrl is not null)
        {
            sb.Append($"repository: {repoUrl}\n");
        }

        if (package.Keywords is { Count: > 0 })
        {
            sb.Append($"keywords: {string.Join(", ", package.Keywords)}\n");
        }

        if (package.Dependencies is { Count: > 0 })
        {
            var deps = package.Dependencies.Keys.Take(10);
            sb.Append($"dependencies: {string.Join(", ", deps)}\n");
            if (package.Dependencies.Count > 10)
            {
                sb.Append($"... and {package.Dependencies.Count - 10} more dependencies\n");
            }
        }

        if (package.Maintainers is { Count: > 0 })
        {
            var names = package.Maintainers.Take(5).Select(FormatAuthor);
            sb.Append($"maintainers: {string.Join(", ", names)}\n");
        }

        if (package.Dist?.UnpackedSize is { } unpackedSize)
        {
            sb.Append($"unpacked-size: {unpackedSize / 1024.0 / 1024.0:F2} MB\n");
        }

        if (package.Dist?.FileCount is { } fileCount)
        {
            sb.Append($"file-count: {fileCount}\n");
        }

        if (package.DistTags?.GetValueOrDefault("latest") is { } latest)
        {
            sb.Append($"latest-version: {latest}\n");
        }

        if (package.DistTags?.GetValueOrDefault("beta") is { } beta)
        {
            sb.Append($"beta-version: {beta}\n");
        }

        var webName = package.Name.StartsWith('@') ? package.Name : Uri.EscapeDataString(package.Name);
        sb.Append($"npm-url: https://www.npmjs.com/package/{webName}\n");
        sb.Append($"registry-url: {RegistryUrl}{EncodeForRegistry(package.Name)}\n");
        sb.Append("source: NPM Registry API\n");
        sb.Append('\n');
        sb.Append("% Information retrieved from NPM registry\n");

        return sb.ToString();
    }

    private static string? FormatRepository(NpmRepository? repository)
    {
        if (repository is null)
        {
            return null;
        }

        return repository switch
        {
            { StringValue: { } s } => s,
            { Url: { } url } => url,
            _ => null,
        };
    }

    private static string FormatAuthor(NpmAuthor author)
    {
        if (author.StringValue is { } s)
        {
            return s;
        }

        return (author.Name, author.Email) switch
        {
            ({ } name, { } email) => $"{name} <{email}>",
            ({ } name, null) => name,
            (null, { } email) => email,
            _ => "Unknown",
        };
    }

    private static string FormatNotFound(string packageName)
    {
        return $"""
            NPM Package Not Found: {packageName}
            No package with this name was found in the NPM registry.

            You can search manually at: https://www.npmjs.com/search?q={Uri.EscapeDataString(packageName)}

            % Package not found in NPM registry
            """;
    }

    private sealed record NpmPackage(
        string Name,
        string Version,
        string? Description,
        string? Homepage,
        NpmRepository? Repository,
        NpmAuthor? Author,
        List<NpmAuthor>? Maintainers,
        string? License,
        List<string>? Keywords,
        Dictionary<string, string>? Dependencies,
        NpmDist? Dist,
        Dictionary<string, string>? DistTags);

    private sealed record NpmRegistryResponse(
        [property: JsonPropertyName("name")] string Name,
        [property: JsonPropertyName("description")] string? Description,
        [property: JsonPropertyName("homepage")] string? Homepage,
        [property: JsonPropertyName("repository")] NpmRepository? Repository,
        [property: JsonPropertyName("author")] NpmAuthor? Author,
        [property: JsonPropertyName("maintainers")] List<NpmAuthor>? Maintainers,
        [property: JsonPropertyName("license")] string? License,
        [property: JsonPropertyName("keywords")] List<string>? Keywords,
        [property: JsonPropertyName("dist-tags")] Dictionary<string, string>? DistTags,
        [property: JsonPropertyName("versions")] Dictionary<string, NpmVersion>? Versions);

    private sealed record NpmVersion(
        [property: JsonPropertyName("description")] string? Description,
        [property: JsonPropertyName("author")] NpmAuthor? Author,
        [property: JsonPropertyName("license")] string? License,
        [property: JsonPropertyName("dependencies")] Dictionary<string, string>? Dependencies,
        [property: JsonPropertyName("keywords")] List<string>? Keywords,
        [property: JsonPropertyName("repository")] NpmRepository? Repository,
        [property: JsonPropertyName("homepage")] string? Homepage,
        [property: JsonPropertyName("dist")] NpmDist? Dist);

    private sealed record NpmDist(
        [property: JsonPropertyName("unpackedSize")] long? UnpackedSize,
        [property: JsonPropertyName("fileCount")] long? FileCount);

    /// <summary>NPM's registry represents <c>author</c>/<c>maintainers</c> entries as either a
    /// bare string ("Jane Doe &lt;jane@example.com&gt;") or an object ({name, email, url}) — mirrors
    /// the Rust version's untagged <c>NPMAuthor</c> enum. The converter below handles both
    /// shapes.</summary>
    [JsonConverter(typeof(NpmAuthorConverter))]
    private sealed record NpmAuthor(string? StringValue, string? Name, string? Email);

    private sealed class NpmAuthorConverter : JsonConverter<NpmAuthor>
    {
        public override NpmAuthor? Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
        {
            if (reader.TokenType == JsonTokenType.String)
            {
                return new NpmAuthor(reader.GetString(), null, null);
            }

            if (reader.TokenType == JsonTokenType.StartObject)
            {
                string? name = null, email = null;
                using var doc = JsonDocument.ParseValue(ref reader);
                if (doc.RootElement.TryGetProperty("name", out var n)) name = n.GetString();
                if (doc.RootElement.TryGetProperty("email", out var e)) email = e.GetString();
                return new NpmAuthor(null, name, email);
            }

            reader.Skip();
            return null;
        }

        public override void Write(Utf8JsonWriter writer, NpmAuthor value, JsonSerializerOptions options) =>
            throw new NotSupportedException();
    }

    /// <summary>NPM's registry represents <c>repository</c> as either a bare URL string or an
    /// object ({type, url, directory}) — mirrors the Rust version's untagged
    /// <c>NPMRepository</c> enum.</summary>
    [JsonConverter(typeof(NpmRepositoryConverter))]
    private sealed record NpmRepository(string? StringValue, string? Url);

    private sealed class NpmRepositoryConverter : JsonConverter<NpmRepository>
    {
        public override NpmRepository? Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
        {
            if (reader.TokenType == JsonTokenType.String)
            {
                return new NpmRepository(reader.GetString(), null);
            }

            if (reader.TokenType == JsonTokenType.StartObject)
            {
                string? url = null;
                using var doc = JsonDocument.ParseValue(ref reader);
                if (doc.RootElement.TryGetProperty("url", out var u)) url = u.GetString();
                return new NpmRepository(null, url);
            }

            reader.Skip();
            return null;
        }

        public override void Write(Utf8JsonWriter writer, NpmRepository value, JsonSerializerOptions options) =>
            throw new NotSupportedException();
    }
}

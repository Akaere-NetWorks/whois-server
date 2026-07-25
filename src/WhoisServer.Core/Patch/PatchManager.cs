using System.Net.Http.Json;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using Microsoft.Extensions.Logging;
using WhoisServer.Core.Storage;

namespace WhoisServer.Core.Patch;

/// <summary>
/// Owns the loaded patch set and the remote SHA1-verified update flow (plan.md §7.1). Unlike the
/// Rust version's global <c>Lazy&lt;RwLock&lt;PatchManager&gt;&gt;</c> plus a spawn_blocking
/// workaround to dodge a Send constraint, this is a normal DI-registered singleton: the loaded
/// patch list is swapped via an atomic reference assignment (safe under .NET's memory model
/// without locking the read path at all), and writes go through a plain <see cref="SemaphoreSlim"/>
/// since nothing here needs to survive across an actual OS thread boundary.
/// </summary>
public sealed class PatchManager(IKvStore store, IHttpClientFactory httpClientFactory, ILogger<PatchManager> logger) : IDisposable
{
    private const string DefaultUpdateUrl =
        "https://raw.githubusercontent.com/Akaere-NetWorks/whois-server/refs/heads/main/patches/patches.json";

    private volatile IReadOnlyList<PatchFile> _patchFiles = [];
    private readonly SemaphoreSlim _updateLock = new(1, 1);

    public void Dispose() => _updateLock.Dispose();

    public (int Files, int Patches) GetCounts()
    {
        var files = _patchFiles;
        return (files.Count, files.Sum(f => f.Patches.Count));
    }

    public string ApplyPatches(string query, string response)
    {
        var files = _patchFiles;
        return files.Count == 0 ? response : PatchApplier.ApplyPatches(query, response, files);
    }

    public async Task<int> LoadFromStorageAsync(CancellationToken ct = default)
    {
        var metaKeys = await store.ListKeysWithPrefixAsync("meta:", ct);
        var names = metaKeys.Select(k => k["meta:".Length..]).OrderBy(n => n, StringComparer.Ordinal).ToList();

        var loaded = new List<PatchFile>();
        foreach (var name in names)
        {
            var content = await store.GetAsync($"patch:{name}", ct);
            if (content is null)
            {
                continue;
            }

            try
            {
                loaded.Add(PatchParser.ParsePatchContent(name, content));
            }
            catch (Exception e)
            {
                logger.LogError(e, "Failed to parse patch {PatchName}", name);
            }
        }

        _patchFiles = loaded;
        var totalPatches = loaded.Sum(f => f.Patches.Count);
        logger.LogInformation("Loaded {PatchCount} patches from {FileCount} files", totalPatches, loaded.Count);
        return totalPatches;
    }

    public async Task<string> UpdateFromRemoteAsync(string? updateUrl = null, CancellationToken ct = default)
    {
        await _updateLock.WaitAsync(ct);
        try
        {
            return await UpdateFromRemoteInternalAsync(updateUrl, ct);
        }
        finally
        {
            _updateLock.Release();
        }
    }

    private async Task<string> UpdateFromRemoteInternalAsync(string? updateUrl, CancellationToken ct)
    {
        var url = updateUrl ?? DefaultUpdateUrl;
        var client = httpClientFactory.CreateClient(nameof(PatchManager));

        logger.LogInformation("Fetching patch metadata from {Url}", url);

        using var request = new HttpRequestMessage(HttpMethod.Get, url);
        request.Headers.TryAddWithoutValidation("Cache-Control", "no-cache, no-store, must-revalidate");
        request.Headers.TryAddWithoutValidation("Pragma", "no-cache");
        request.Headers.TryAddWithoutValidation("Expires", "0");

        using var response = await client.SendAsync(request, ct);
        response.EnsureSuccessStatusCode();
        var metadata = await response.Content.ReadFromJsonAsync<PatchMetadata>(cancellationToken: ct)
            ?? throw new InvalidOperationException("Patch metadata response was empty");

        var output = new StringBuilder();
        output.AppendLine("% Patch Update Report");
        output.AppendLine($"% Downloaded from: {url}");
        output.AppendLine($"% Last Updated: {metadata.LastUpdated}");
        output.AppendLine($"% Format Version: {metadata.Metadata.FormatVersion}");
        output.AppendLine("%");

        var successCount = 0;
        var failedCount = 0;
        var skippedCount = 0;

        foreach (var patchInfo in metadata.Patches)
        {
            if (!patchInfo.Enabled)
            {
                continue;
            }

            output.AppendLine();
            output.AppendLine($"patch:           {patchInfo.Name}");
            output.AppendLine($"description:     {patchInfo.Description}");
            output.AppendLine($"url:             {patchInfo.Url}");
            output.AppendLine($"sha1-expected:   {patchInfo.Sha1}");
            output.AppendLine($"size-expected:   {patchInfo.Size} bytes");
            output.AppendLine($"priority:        {patchInfo.Priority}");
            output.AppendLine($"modified:        {patchInfo.Modified}");

            try
            {
                var (actualSha1, wasUpdated) = await DownloadAndVerifyPatchAsync(client, patchInfo, ct);
                output.AppendLine($"sha1-actual:     {actualSha1}");

                if (actualSha1 == patchInfo.Sha1)
                {
                    if (wasUpdated)
                    {
                        output.AppendLine("status:          ✓ VERIFIED (downloaded)");
                        successCount++;
                    }
                    else
                    {
                        output.AppendLine("status:          ✓ UP-TO-DATE (skipped)");
                        skippedCount++;
                    }
                }
                else
                {
                    output.AppendLine("status:          ✗ SHA1 MISMATCH");
                    failedCount++;
                }
            }
            catch (Exception e)
            {
                output.AppendLine("status:          ✗ FAILED");
                output.AppendLine($"error:           {e.Message}");
                failedCount++;
            }
        }

        output.AppendLine();
        output.AppendLine("% Summary");
        output.AppendLine($"% Total patches: {metadata.Patches.Count}");
        output.AppendLine($"% Downloaded: {successCount}");
        output.AppendLine($"% Up-to-date (skipped): {skippedCount}");
        output.AppendLine($"% Failed: {failedCount}");
        output.AppendLine("%");

        if (successCount > 0 || skippedCount > 0)
        {
            var count = await LoadFromStorageAsync(ct);
            output.AppendLine($"% Patches reloaded: {count} patch files loaded into memory");
        }

        output.AppendLine("%");
        output.AppendLine("% Run 'whois help' for more information");

        return output.ToString();
    }

    private async Task<(string ActualSha1, bool WasUpdated)> DownloadAndVerifyPatchAsync(
        HttpClient client, PatchInfo patchInfo, CancellationToken ct)
    {
        var metaKey = $"meta:{patchInfo.Name}";
        var existingMetaJson = await store.GetAsync(metaKey, ct);
        if (existingMetaJson is not null)
        {
            var existingInfo = JsonSerializer.Deserialize<PatchInfo>(existingMetaJson);
            if (existingInfo is not null && existingInfo.Sha1 == patchInfo.Sha1)
            {
                return (patchInfo.Sha1, false);
            }
        }

        using var request = new HttpRequestMessage(HttpMethod.Get, patchInfo.Url);
        request.Headers.TryAddWithoutValidation("Cache-Control", "no-cache, no-store, must-revalidate");
        request.Headers.TryAddWithoutValidation("Pragma", "no-cache");
        request.Headers.TryAddWithoutValidation("Expires", "0");

        using var response = await client.SendAsync(request, ct);
        response.EnsureSuccessStatusCode();
        var content = await response.Content.ReadAsStringAsync(ct);

        // SHA1 here matches the pre-existing patches.json manifest schema's own "sha1-expected"
        // field (a content-addressable integrity check against the same trusted source the
        // manifest itself came from, not a defense against an adversarial hash) — not a security
        // boundary this code controls, and changing algorithms would break manifest compatibility.
#pragma warning disable CA5350
        var actualSha1 = Convert.ToHexStringLower(SHA1.HashData(Encoding.UTF8.GetBytes(content)));
#pragma warning restore CA5350

        if (actualSha1 == patchInfo.Sha1)
        {
            await store.PutAsync($"patch:{patchInfo.Name}", content, ct: ct);
            await store.PutAsync(metaKey, JsonSerializer.Serialize(patchInfo), ct: ct);
            return (actualSha1, true);
        }

        return (actualSha1, false);
    }
}

#if FEATURE_DN42
using System.Diagnostics;
using Microsoft.Extensions.Logging;
using WhoisServer.Core.Storage;

namespace WhoisServer.Services.Dn42;

/// <summary>
/// Ported from src/dn42/git_backend.rs's clone/pull/populate flow. Clones (shallow, --depth 1) or
/// pulls (fetch + hard-reset to origin/master, falling back to origin/main) the DN42 registry
/// mirror by shelling out to the system `git` binary — see the project's .csproj comment for why
/// this was chosen over LibGit2Sharp. Then walks data/&lt;object-type&gt;/&lt;file&gt; and stores
/// each file's raw content verbatim under key "&lt;object-type&gt;/&lt;file&gt;", skipping files
/// whose (size, mtime) haven't changed since the last sync — the same incremental-skip strategy
/// the Rust version uses to make repeated hourly syncs cheap.
/// </summary>
public sealed class Dn42RegistrySync(string registryPath, IKvStore store, ILogger<Dn42RegistrySync> logger)
{
    // Resolved once, up front: git's target-directory argument and the .NET-side directory
    // checks must agree on the *same* path. Using the raw (possibly relative) constructor
    // argument for one and a `Process.WorkingDirectory`-relative resolution for the other made
    // the clone silently land in a nested "cache/cache/dn42-registry" the first time this ran —
    // caught by an actual end-to-end run, not by unit tests (nothing here is relative-path-order
    // sensitive enough for a unit test to catch it; see plan.md's Phase 3 notes).
    private readonly string _registryPath = Path.GetFullPath(registryPath);

    public async Task SyncAsync(CancellationToken ct = default)
    {
        await Task.Run(SyncRegistryOnDisk, ct);
        await PopulateAsync(ct);
    }

    private void SyncRegistryOnDisk()
    {
        var gitDir = Path.Combine(_registryPath, ".git");

        if (Directory.Exists(_registryPath) && Directory.Exists(gitDir))
        {
            logger.LogInformation("DN42 registry exists, pulling latest changes");
            PullLatestChanges();
            return;
        }

        if (Directory.Exists(_registryPath))
        {
            logger.LogWarning("DN42 registry directory exists but isn't a git repository — re-cloning");
            Directory.Delete(_registryPath, recursive: true);
        }

        logger.LogInformation("Cloning DN42 registry from {Url}", Dn42Constants.RegistryUrl);
        CloneRepository();
    }

    private void CloneRepository()
    {
        var parent = Path.GetDirectoryName(_registryPath);
        if (!string.IsNullOrEmpty(parent))
        {
            Directory.CreateDirectory(parent);
        }

        RunGit(parent ?? ".", "clone", "--depth", "1", Dn42Constants.RegistryUrl, _registryPath);

        if (!Directory.Exists(Path.Combine(_registryPath, "data")))
        {
            throw new InvalidOperationException($"Cloned DN42 registry is missing the data directory: {_registryPath}");
        }
    }

    private void PullLatestChanges()
    {
        RunGit(_registryPath, "fetch", "origin");

        try
        {
            RunGit(_registryPath, "reset", "--hard", "origin/master");
        }
        catch (Exception masterEx)
        {
            logger.LogDebug(masterEx, "Reset to origin/master failed, trying origin/main");
            RunGit(_registryPath, "reset", "--hard", "origin/main");
        }
    }

    private static void RunGit(string workingDirectory, params string[] args)
    {
        using var process = new Process
        {
            StartInfo = new ProcessStartInfo("git")
            {
                WorkingDirectory = workingDirectory,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
            },
        };

        foreach (var arg in args)
        {
            process.StartInfo.ArgumentList.Add(arg);
        }

        process.Start();
        var stderr = process.StandardError.ReadToEnd();
        process.WaitForExit();

        if (process.ExitCode != 0)
        {
            throw new InvalidOperationException($"git {string.Join(' ', args)} failed (exit {process.ExitCode}): {stderr}");
        }
    }

    private async Task PopulateAsync(CancellationToken ct)
    {
        var dataDir = Path.Combine(_registryPath, "data");
        if (!Directory.Exists(dataDir))
        {
            throw new InvalidOperationException($"DN42 registry data directory not found: {dataDir}");
        }

        var updatedCount = 0;
        var totalCount = 0;

        foreach (var subdir in Directory.GetDirectories(dataDir))
        {
            var objectType = Path.GetFileName(subdir);

            foreach (var file in Directory.GetFiles(subdir))
            {
                ct.ThrowIfCancellationRequested();
                totalCount++;

                var fileName = Path.GetFileName(file);
                var key = $"{objectType}/{fileName}";
                var metaKey = $"__meta__{key}";

                var info = new FileInfo(file);
                var metaValue = $"{info.Length}:{info.LastWriteTimeUtc.Ticks}";

                if (await store.GetAsync(metaKey, ct) == metaValue)
                {
                    continue; // unchanged since the last sync
                }

                var content = await File.ReadAllTextAsync(file, ct);
                await store.PutAsync(key, content, ct: ct);
                await store.PutAsync(metaKey, metaValue, ct: ct);
                updatedCount++;
            }
        }

        logger.LogInformation(
            "DN42 registry sync: {Updated}/{Total} object files updated", updatedCount, totalCount);
    }
}
#endif

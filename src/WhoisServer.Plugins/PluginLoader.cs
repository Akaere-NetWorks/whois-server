using System.Reflection;
using System.Text.Json;
using Microsoft.Extensions.Logging;
using WhoisServer.Core.Query;
using WhoisServer.Core.Storage;

namespace WhoisServer.Plugins;

/// <summary>
/// Scans a <c>plugins/&lt;name&gt;/</c> directory tree for <c>plugin.json</c> manifests, loads
/// each enabled plugin's assembly into its own <see cref="PluginLoadContext"/>, resolves its
/// constructor dependencies, and registers it into a <see cref="PluginRegistry"/> and
/// <see cref="QueryAnalyzer"/>'s suffix registry — the native-C# replacement for
/// <c>src/plugins/loader.rs</c>'s <c>load_all_plugins</c>.
/// </summary>
public static class PluginLoader
{
    private static readonly JsonSerializerOptions ManifestJsonOptions = new() { PropertyNameCaseInsensitive = true };

    /// <returns>The number of plugins successfully loaded.</returns>
    public static async Task<int> LoadAllAsync(
        PluginRegistry registry,
        string pluginsDir,
        IKvStore sharedCache,
        ILoggerFactory loggerFactory,
        CancellationToken ct = default)
    {
        var logger = loggerFactory.CreateLogger("WhoisServer.Plugins.PluginLoader");

        if (!Directory.Exists(pluginsDir))
        {
            logger.LogInformation("Creating plugins directory at {Dir}", pluginsDir);
            Directory.CreateDirectory(pluginsDir);
            return 0;
        }

        var envVars = LoadEnvFile(Path.Combine(pluginsDir, ".plugins.env"));

        var loaded = 0;
        var skipped = 0;

        foreach (var dir in Directory.EnumerateDirectories(pluginsDir))
        {
            ct.ThrowIfCancellationRequested();
            try
            {
                var host = await LoadPluginFromDirAsync(dir, envVars, sharedCache, loggerFactory, ct);
                if (host is null)
                {
                    skipped++;
                    continue;
                }

                if (QueryAnalyzer.IsSuffixRegistered(host.Suffix) || registry.IsSuffixTaken(host.Suffix))
                {
                    logger.LogWarning(
                        "Skipping plugin '{Name}' from {Dir}: suffix '{Suffix}' is already registered",
                        host.Name, dir, host.Suffix);
                    host.Unload();
                    skipped++;
                    continue;
                }

                var suffix = host.Suffix;
                QueryAnalyzer.RegisterSuffix(suffix, baseQuery => new QueryType.Plugin(suffix, baseQuery));
                registry.Register(host);

                logger.LogInformation(
                    "Loaded plugin '{Name}' v{Version} (suffix: {Suffix})", host.Name, host.Manifest.Version, host.Suffix);
                loaded++;
            }
            catch (Exception e)
            {
                logger.LogWarning(e, "Failed to load plugin from {Dir}", dir);
                skipped++;
            }
        }

        logger.LogInformation("Loaded {Loaded} plugin(s), skipped {Skipped}", loaded, skipped);
        return loaded;
    }

    private static async Task<PluginHost?> LoadPluginFromDirAsync(
        string dir, IReadOnlyDictionary<string, string> allEnvVars, IKvStore sharedCache,
        ILoggerFactory loggerFactory, CancellationToken ct)
    {
        var manifestPath = Path.Combine(dir, "plugin.json");
        if (!File.Exists(manifestPath))
        {
            throw new FileNotFoundException("plugin.json not found", manifestPath);
        }

        var manifestJson = await File.ReadAllTextAsync(manifestPath, ct);
        var manifest = JsonSerializer.Deserialize<PluginManifest>(manifestJson, ManifestJsonOptions)
            ?? throw new InvalidOperationException("plugin.json parsed to null");

        if (string.IsNullOrEmpty(manifest.Name) || string.IsNullOrEmpty(manifest.Version))
        {
            throw new InvalidOperationException("plugin.json must set non-empty name and version");
        }

        if (!manifest.Enabled)
        {
            throw new InvalidOperationException("Plugin is disabled in plugin.json");
        }

        if (!manifest.Suffix.StartsWith('-'))
        {
            throw new InvalidOperationException($"Plugin suffix must start with '-', got: {manifest.Suffix}");
        }

        var entryPoint = manifest.EntryPoint ?? $"{Path.GetFileName(dir.TrimEnd(Path.DirectorySeparatorChar))}.dll";
        // AssemblyDependencyResolver (used by PluginLoadContext) throws ArgumentException on a
        // relative path — surfaced only when pluginsDir itself is relative (the default, matching
        // the Rust version's own relative "plugins" convention), since every test here happened to
        // pass an absolute temp-directory path and never hit it.
        var dllPath = Path.GetFullPath(Path.Combine(dir, entryPoint));
        if (!File.Exists(dllPath))
        {
            throw new FileNotFoundException($"Plugin entry point assembly not found: {entryPoint}", dllPath);
        }

        var loadContext = new PluginLoadContext(dllPath);
        var assembly = loadContext.LoadFromAssemblyPath(dllPath);

        var pluginType = assembly.GetTypes()
            .Where(t => t is { IsPublic: true, IsAbstract: false } && typeof(IWhoisPlugin).IsAssignableFrom(t))
            .ToList();

        if (pluginType.Count != 1)
        {
            throw new InvalidOperationException(
                $"Expected exactly one public class implementing IWhoisPlugin in {entryPoint}, found {pluginType.Count}");
        }

        var pluginEnvVars = manifest.Permissions.EnvVars.Count == 0
            ? new Dictionary<string, string>()
            : manifest.Permissions.EnvVars
                .Where(allEnvVars.ContainsKey)
                .ToDictionary(k => k, k => allEnvVars[k]);

        var instance = Instantiate(pluginType[0], manifest, pluginEnvVars, sharedCache, loggerFactory);
        return new PluginHost(manifest, instance, loadContext);
    }

    /// <summary>Minimal constructor-injection resolver — plugins may request
    /// <see cref="HttpClient"/>, <see cref="IKvStore"/>, <see cref="ILogger"/> (or
    /// <c>ILogger&lt;TSelf&gt;</c>), and/or <c>IReadOnlyDictionary&lt;string, string&gt;</c>
    /// (env vars); nothing else. There's no general IoC container involved — a plugin assembly is
    /// loaded into its own <see cref="PluginLoadContext"/>, so it can't share the host's
    /// <c>IServiceProvider</c> registrations even if it wanted to.</summary>
    private static IWhoisPlugin Instantiate(
        Type pluginType, PluginManifest manifest, IReadOnlyDictionary<string, string> envVars,
        IKvStore sharedCache, ILoggerFactory loggerFactory)
    {
        var constructors = pluginType.GetConstructors();
        if (constructors.Length != 1)
        {
            throw new InvalidOperationException(
                $"Plugin type {pluginType.FullName} must have exactly one public constructor, found {constructors.Length}");
        }

        var parameters = constructors[0].GetParameters();
        var args = new object?[parameters.Length];

        for (var i = 0; i < parameters.Length; i++)
        {
            var paramType = parameters[i].ParameterType;

            if (paramType == typeof(HttpClient))
            {
                if (!manifest.Permissions.Network)
                {
                    throw new InvalidOperationException(
                        $"Plugin constructor requests HttpClient but plugin.json permissions.network is false");
                }

                var handler = new DomainWhitelistHttpHandler(manifest.Permissions.AllowedDomains);
                var client = new HttpClient(handler);
                client.DefaultRequestHeaders.UserAgent.ParseAdd(
                    manifest.Permissions.UserAgent ?? $"whois-server-plugin/{manifest.Name}");
                args[i] = client;
            }
            else if (paramType == typeof(IKvStore))
            {
                if (!manifest.Permissions.CacheRead && !manifest.Permissions.CacheWrite)
                {
                    throw new InvalidOperationException(
                        "Plugin constructor requests IKvStore but plugin.json grants neither cacheRead nor cacheWrite");
                }

                args[i] = new NamespacedKvStore(
                    sharedCache, manifest.Name, manifest.Permissions.CacheRead, manifest.Permissions.CacheWrite);
            }
            else if (paramType == typeof(ILogger) ||
                     (paramType.IsGenericType && paramType.GetGenericTypeDefinition() == typeof(ILogger<>)))
            {
                args[i] = loggerFactory.CreateLogger($"Plugin.{manifest.Name}");
            }
            else if (paramType == typeof(IReadOnlyDictionary<string, string>))
            {
                args[i] = envVars;
            }
            else
            {
                throw new InvalidOperationException(
                    $"Unsupported plugin constructor parameter type '{paramType}' — plugins may only take " +
                    "HttpClient, IKvStore, ILogger/ILogger<T>, or IReadOnlyDictionary<string, string>");
            }
        }

        return (IWhoisPlugin)constructors[0].Invoke(args);
    }

    /// <summary>Same <c>KEY=VALUE</c>-per-line format as the Rust version's <c>.plugins.env</c>
    /// loader (comments with <c>#</c>, blank lines ignored, optional surrounding quotes stripped).</summary>
    private static Dictionary<string, string> LoadEnvFile(string path)
    {
        var result = new Dictionary<string, string>();
        if (!File.Exists(path))
        {
            return result;
        }

        foreach (var rawLine in File.ReadLines(path))
        {
            var line = rawLine.Trim();
            if (line.Length == 0 || line.StartsWith('#'))
            {
                continue;
            }

            var eq = line.IndexOf('=');
            if (eq < 0)
            {
                continue;
            }

            var key = line[..eq].Trim();
            var value = line[(eq + 1)..].Trim();
            if (value.Length >= 2 && ((value[0] == '"' && value[^1] == '"') || (value[0] == '\'' && value[^1] == '\'')))
            {
                value = value[1..^1];
            }

            if (key.Length > 0)
            {
                result[key] = value;
            }
        }

        return result;
    }
}

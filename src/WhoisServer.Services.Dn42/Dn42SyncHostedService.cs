#if FEATURE_DN42
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace WhoisServer.Services.Dn42;

/// <summary>Initial sync at startup, then hourly — ported from src/dn42/git_backend.rs's
/// <c>start_periodic_sync</c>. A failed sync (network down, git missing, etc.) logs and moves on;
/// DN42 queries just keep 404ing against whatever was last successfully synced (or nothing, on a
/// completely fresh install with no successful sync yet) rather than crashing the whole host.</summary>
public sealed class Dn42SyncHostedService(Dn42RegistrySync sync, ILogger<Dn42SyncHostedService> logger) : BackgroundService
{
    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        await RunSyncSafelyAsync("initial", stoppingToken);

        using var timer = new PeriodicTimer(Dn42Constants.SyncInterval);
        try
        {
            while (await timer.WaitForNextTickAsync(stoppingToken))
            {
                await RunSyncSafelyAsync("scheduled", stoppingToken);
            }
        }
        catch (OperationCanceledException)
        {
            // Expected on shutdown.
        }
    }

    private async Task RunSyncSafelyAsync(string label, CancellationToken ct)
    {
        try
        {
            logger.LogInformation("Starting {Label} DN42 registry sync", label);
            await sync.SyncAsync(ct);
            logger.LogInformation("Completed {Label} DN42 registry sync", label);
        }
        catch (Exception e)
        {
            logger.LogError(e, "{Label} DN42 registry sync failed", label);
        }
    }
}
#endif

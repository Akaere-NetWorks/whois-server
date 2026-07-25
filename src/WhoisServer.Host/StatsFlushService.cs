using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using WhoisServer.Core.Stats;

namespace WhoisServer.Host;

/// <summary>
/// Periodically persists and prunes statistics, plus a final flush on shutdown. Replaces the Rust
/// version's every-10th/every-100th-request modulo counters (an artifact of not having graceful
/// shutdown wired up — see plan.md §4.4/§4.6/§7.8) with a time-based background flush, and — since
/// this runs as a normal <see cref="BackgroundService"/> under the Generic Host — a real shutdown
/// flush that fires whenever the host stops (including Ctrl+C), not only on a fatal accept-loop
/// error like the original.
/// </summary>
public sealed class StatsFlushService(StatsManager stats, ILogger<StatsFlushService> logger) : BackgroundService
{
    private static readonly TimeSpan FlushInterval = TimeSpan.FromSeconds(30);
    private static readonly TimeSpan CleanupInterval = TimeSpan.FromMinutes(30);

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        await stats.LoadAsync(stoppingToken);

        var lastCleanup = DateTimeOffset.UtcNow;
        using var timer = new PeriodicTimer(FlushInterval);

        try
        {
            while (await timer.WaitForNextTickAsync(stoppingToken))
            {
                await stats.SaveAsync(stoppingToken);

                if (DateTimeOffset.UtcNow - lastCleanup > CleanupInterval)
                {
                    stats.CleanupOld();
                    lastCleanup = DateTimeOffset.UtcNow;
                }
            }
        }
        catch (OperationCanceledException)
        {
            // Expected on shutdown.
        }
        finally
        {
            logger.LogInformation("Flushing statistics on shutdown");
            await stats.SaveAsync(CancellationToken.None);
        }
    }
}

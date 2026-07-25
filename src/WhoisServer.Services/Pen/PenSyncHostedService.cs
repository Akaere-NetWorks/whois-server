using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace WhoisServer.Services.Pen;

/// <summary>Checks hourly (initial check at startup) whether the PEN registry cache is more than
/// a day old and re-downloads/re-parses it if so — ported from pen.rs's
/// <c>start_pen_periodic_update</c>. A failed refresh logs and moves on; PEN queries just keep
/// serving whatever was last cached.</summary>
public sealed class PenSyncHostedService(PenService pen, ILogger<PenSyncHostedService> logger) : BackgroundService
{
    private static readonly TimeSpan CheckInterval = TimeSpan.FromHours(1);

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        await RefreshIfNeededSafelyAsync("initial", stoppingToken);

        using var timer = new PeriodicTimer(CheckInterval);
        try
        {
            while (await timer.WaitForNextTickAsync(stoppingToken))
            {
                await RefreshIfNeededSafelyAsync("scheduled", stoppingToken);
            }
        }
        catch (OperationCanceledException)
        {
            // Expected on shutdown.
        }
    }

    private async Task RefreshIfNeededSafelyAsync(string label, CancellationToken ct)
    {
        try
        {
            if (!await pen.NeedsUpdateAsync(ct))
            {
                logger.LogDebug("PEN cache is up to date ({Label} check)", label);
                return;
            }

            logger.LogInformation("Starting {Label} PEN registry update", label);
            await pen.ForceUpdateAsync(ct);
            logger.LogInformation("Completed {Label} PEN registry update", label);
        }
        catch (Exception e)
        {
            logger.LogError(e, "{Label} PEN registry update failed", label);
        }
    }
}

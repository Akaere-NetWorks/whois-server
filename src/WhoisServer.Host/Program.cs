using Microsoft.Extensions.Hosting;
using WhoisServer.Host;

CliOptions cliOptions;
try
{
    cliOptions = CliParser.Parse(args);
}
catch (HelpRequestedException)
{
    PrintUsage();
    return 0;
}
catch (ArgumentException e)
{
    Console.Error.WriteLine(e.Message);
    PrintUsage();
    return 1;
}

var host = HostFactory.Build(cliOptions, "./cache", args);
await HostFactory.LoadPatchesAsync(host);
await HostFactory.LoadPluginsAsync(host);
await host.RunAsync();
return 0;

static void PrintUsage()
{
    Console.WriteLine(
        """
        A WHOIS server with comprehensive query capabilities

        Options:
          -H, --host <HOST>              Listen address [default: 0.0.0.0]
          -p, --port <PORT>              WHOIS server port [default: 43]
          -d, --debug                    Enable debug output
          -t, --trace                    Enable trace output (extremely verbose)
              --max-connections <N>      Maximum concurrent connections [default: 100]
              --timeout <SECONDS>        Connection timeout in seconds [default: 10]
              --dump-traffic             Write raw queries and responses to files for debugging
              --dump-dir <DIR>           Dump traffic directory [default: dumps]
              --web-port <PORT>          Web dashboard port [default: 9999]
              --enable-color             Enable WHOIS-COLOR protocol support [default: true]
              --help                     Print help
        """);
}

/// <summary>Entry point type, needed as a generic-logger-category anchor. The compiler-generated
/// top-level-statement class is implicitly `partial` (and internal), so this must match — not
/// `public` — or the two partial declarations conflict.</summary>
partial class Program;

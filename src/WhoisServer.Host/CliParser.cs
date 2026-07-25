namespace WhoisServer.Host;

/// <summary>
/// Hand-rolled parser for the small, fixed flag set in <see cref="CliOptions"/> — mirrors the
/// Rust version's `clap`-derived flags (minus SSH). Supports `--flag value`, `--flag=value`, and
/// the handful of short forms (`-H`, `-p`, `-d`, `-t`) the original exposed.
/// </summary>
public static class CliParser
{
    public static CliOptions Parse(string[] args)
    {
        var options = new CliOptions();

        for (var i = 0; i < args.Length; i++)
        {
            var (flag, inlineValue) = SplitFlag(args[i]);

            string NextValue()
            {
                if (inlineValue is not null)
                {
                    return inlineValue;
                }

                if (i + 1 >= args.Length)
                {
                    throw new ArgumentException($"Missing value for {flag}");
                }

                return args[++i];
            }

            options = flag switch
            {
                "--host" or "-H" => options with { Host = NextValue() },
                "--port" or "-p" => options with { Port = int.Parse(NextValue()) },
                "--debug" or "-d" => options with { Debug = true },
                "--trace" or "-t" => options with { Trace = true },
                "--max-connections" => options with { MaxConnections = int.Parse(NextValue()) },
                "--timeout" => options with { TimeoutSeconds = int.Parse(NextValue()) },
                "--dump-traffic" => options with { DumpTraffic = true },
                "--dump-dir" => options with { DumpDir = NextValue() },
                "--web-port" => options with { WebPort = int.Parse(NextValue()) },
                "--enable-color" => options with { EnableColor = ParseBoolValue(inlineValue) },
                "--help" or "-h" => throw new HelpRequestedException(),
                _ => throw new ArgumentException($"Unknown option: {flag}"),
            };
        }

        return options;

        static bool ParseBoolValue(string? inlineValue) => inlineValue is null || bool.Parse(inlineValue);
    }

    private static (string Flag, string? InlineValue) SplitFlag(string arg)
    {
        var eqIndex = arg.IndexOf('=');
        return eqIndex < 0 ? (arg, null) : (arg[..eqIndex], arg[(eqIndex + 1)..]);
    }
}

public sealed class HelpRequestedException : Exception;

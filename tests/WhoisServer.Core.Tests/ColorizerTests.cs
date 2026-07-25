using System.Net;
using WhoisServer.Core.Color;
using WhoisServer.Core.Query;
using Xunit;

namespace WhoisServer.Core.Tests;

/// <summary>Ported from src/core/color/tests.rs — golden tests for the colorizer port (plan.md §7.2/§8).</summary>
public class ColorizerTests
{
    [Fact]
    public void ColorSchemeParsing()
    {
        Assert.Equal(ColorScheme.Ripe, ColorSchemeExtensions.FromString("ripe"));
        Assert.Equal(ColorScheme.Ripe, ColorSchemeExtensions.FromString("RIPE"));
        Assert.Equal(ColorScheme.BgpTools, ColorSchemeExtensions.FromString("bgptools"));
        Assert.Equal(ColorScheme.RipeDark, ColorSchemeExtensions.FromString("ripe-dark"));
        Assert.Equal(ColorScheme.RipeDark, ColorSchemeExtensions.FromString("dark-ripe"));
        Assert.Equal(ColorScheme.BgpToolsDark, ColorSchemeExtensions.FromString("bgptools-dark"));
        Assert.Equal(ColorScheme.BgpToolsDark, ColorSchemeExtensions.FromString("dark-bgptools"));
        Assert.Null(ColorSchemeExtensions.FromString("invalid"));
    }

    [Fact]
    public void ProtocolHeaderParsing()
    {
        var protocol = new ColorProtocol();
        var probeRequest = "X-WHOIS-COLOR-PROBE: 1\r\nexample.com\r\n";
        Assert.True(protocol.ParseHeaders(probeRequest));
        Assert.True(protocol.ClientSupportsColor);

        var protocol2 = new ColorProtocol();
        var colorRequest = "X-WHOIS-COLOR: ripe\r\nexample.com\r\n";
        Assert.False(protocol2.ParseHeaders(colorRequest));
        Assert.True(protocol2.ClientSupportsColor);
        Assert.Equal(ColorScheme.Ripe, protocol2.Scheme);

        var protocol3 = new ColorProtocol();
        var darkRequest = "X-WHOIS-COLOR: ripe-dark\r\nexample.com\r\n";
        Assert.False(protocol3.ParseHeaders(darkRequest));
        Assert.True(protocol3.ClientSupportsColor);
        Assert.Equal(ColorScheme.RipeDark, protocol3.Scheme);

        var protocol4 = new ColorProtocol();
        var schemeRequest = "X-WHOIS-COLOR: scheme=bgptools-dark\r\nexample.com\r\n";
        Assert.False(protocol4.ParseHeaders(schemeRequest));
        Assert.True(protocol4.ClientSupportsColor);
        Assert.Equal(ColorScheme.BgpToolsDark, protocol4.Scheme);
    }

    [Fact]
    public void CapabilityResponse()
    {
        var protocol = new ColorProtocol();
        var response = protocol.GetCapabilityResponse();
        Assert.Contains("ripe", response);
        Assert.Contains("ripe-dark", response);
        Assert.Contains("bgptools", response);
        Assert.Contains("bgptools-dark", response);
    }

    [Fact]
    public void DarkVsLightColoring()
    {
        const string sample = "% Test\ninetnum: 192.0.2.0 - 192.0.2.255\nnetname: EXAMPLE-NET\ndescr: Example network";

        var ripeColorizer = new Colorizer(ColorScheme.Ripe);
        var ripeDarkColorizer = new Colorizer(ColorScheme.RipeDark);

        var queryType = new QueryType.IPv4(IPAddress.Parse("192.0.2.0"));
        var lightOutput = ripeColorizer.ColorizeResponse(sample, queryType);
        var darkOutput = ripeDarkColorizer.ColorizeResponse(sample, queryType);

        Assert.NotEqual(lightOutput, darkOutput);

        // Dark mode should use dim white for comments, light mode should use bright black.
        Assert.Contains("\x1b[90m", lightOutput);
        Assert.Contains("\x1b[37m", darkOutput);
    }

    [Fact]
    public void BgpToolsDarkColoring()
    {
        const string sample = "% Test\norigin: AS64544\nroute: 192.0.2.0/24";

        var bgpToolsColorizer = new Colorizer(ColorScheme.BgpTools);
        var bgpToolsDarkColorizer = new Colorizer(ColorScheme.BgpToolsDark);

        // BGPTool query type doesn't exist yet in this phase's QueryType hierarchy (added when
        // the BGP services land) — Unknown exercises the same "general highlighting" fallback path.
        var queryType = new QueryType.Unknown("192.0.2.0/24");
        var lightOutput = bgpToolsColorizer.ColorizeResponse(sample, queryType);
        var darkOutput = bgpToolsDarkColorizer.ColorizeResponse(sample, queryType);

        Assert.NotEqual(lightOutput, darkOutput);
    }
}

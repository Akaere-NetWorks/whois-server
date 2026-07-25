namespace WhoisServer.Core.Color;

public enum ColorScheme
{
    Ripe,
    RipeDark,
    BgpTools,
    BgpToolsDark,
}

public static class ColorSchemeExtensions
{
    public static ColorScheme? FromString(string s)
    {
        return s.ToLowerInvariant() switch
        {
            "ripe" => ColorScheme.Ripe,
            "ripe-dark" or "dark-ripe" => ColorScheme.RipeDark,
            "bgptools" => ColorScheme.BgpTools,
            "bgptools-dark" or "dark-bgptools" => ColorScheme.BgpToolsDark,
            _ => null,
        };
    }
}

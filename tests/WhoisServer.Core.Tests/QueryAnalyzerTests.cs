using System.Net;
using WhoisServer.Core.Query;
using Xunit;

namespace WhoisServer.Core.Tests;

public class QueryAnalyzerTests
{
    [Fact]
    public void ClassifiesDomain()
    {
        var result = QueryAnalyzer.Analyze("example.com");
        var domain = Assert.IsType<QueryType.Domain>(result);
        Assert.Equal("example.com", domain.Name);
    }

    [Fact]
    public void ClassifiesIPv4()
    {
        var result = QueryAnalyzer.Analyze("8.8.8.8");
        var ip = Assert.IsType<QueryType.IPv4>(result);
        Assert.Equal(IPAddress.Parse("8.8.8.8"), ip.Address);
    }

    [Fact]
    public void ClassifiesIPv6()
    {
        var result = QueryAnalyzer.Analyze("2001:4860:4860::8888");
        var ip = Assert.IsType<QueryType.IPv6>(result);
        Assert.Equal(IPAddress.Parse("2001:4860:4860::8888"), ip.Address);
    }

    [Fact]
    public void ClassifiesIPv4Cidr_UsingFirstAddress()
    {
        var result = QueryAnalyzer.Analyze("192.0.2.128/25");
        var ip = Assert.IsType<QueryType.IPv4>(result);
        Assert.Equal(IPAddress.Parse("192.0.2.128"), ip.Address);
    }

    [Fact]
    public void ClassifiesAsn()
    {
        var result = QueryAnalyzer.Analyze("AS213605");
        var asn = Assert.IsType<QueryType.Asn>(result);
        Assert.Equal("AS213605", asn.Value);
    }

    [Fact]
    public void ClassifiesBareAsn()
    {
        var result = QueryAnalyzer.Analyze("213605");
        var asn = Assert.IsType<QueryType.BareAsn>(result);
        Assert.Equal("213605", asn.Value);
    }

    [Theory]
    [InlineData("HELP")]
    [InlineData("help")]
    [InlineData("Help")]
    public void ClassifiesHelp(string query)
    {
        Assert.IsType<QueryType.Help>(QueryAnalyzer.Analyze(query));
    }

    [Theory]
    [InlineData("UPDATE-PATCH")]
    [InlineData("-UPDATE-PATCH")]
    [InlineData("update-patch")]
    public void ClassifiesUpdatePatch(string query)
    {
        Assert.IsType<QueryType.UpdatePatch>(QueryAnalyzer.Analyze(query));
    }

    [Fact]
    public void ClassifiesDescSuffix_PreservingOriginalCase()
    {
        var result = QueryAnalyzer.Analyze("Example.COM-desc");
        var desc = Assert.IsType<QueryType.Desc>(result);
        Assert.Equal("Example.COM", desc.BaseQuery);
    }

    [Fact]
    public void ClassifiesRdapSuffix()
    {
        var result = QueryAnalyzer.Analyze("AS213605-RDAP");
        var rdap = Assert.IsType<QueryType.Rdap>(result);
        Assert.Equal("AS213605", rdap.BaseQuery);
    }

    [Fact]
    public void ClassifiesDn42Domain_EvenThoughItWouldNotMatchTheDomainRegex()
    {
        // "dn42" contains digits, so the generic domain regex's letters-only TLD group would
        // reject it — this only classifies as Domain because of the explicit .dn42 check.
        var result = QueryAnalyzer.Analyze("example.dn42");
        var domain = Assert.IsType<QueryType.Domain>(result);
        Assert.Equal("example.dn42", domain.Name);
    }

    [Theory]
    [InlineData("EXAMPLE-DN42")]
    [InlineData("EXAMPLE-MNT")]
    public void ClassifiesDn42PersonAndMaintainerHandlesAsUnknown(string query)
    {
        var result = QueryAnalyzer.Analyze(query);
        var unknown = Assert.IsType<QueryType.Unknown>(result);
        Assert.Equal(query, unknown.Raw);
    }

    [Fact]
    public void ClassifiesUnknownForGarbage()
    {
        Assert.IsType<QueryType.Unknown>(QueryAnalyzer.Analyze("!!!not-a-real-query###"));
    }

    [Fact]
    public void ClassifiesRpki_WithCidrPrefix()
    {
        var result = QueryAnalyzer.Analyze("192.0.2.0/24-213605-RPKI");
        var rpki = Assert.IsType<QueryType.Rpki>(result);
        Assert.Equal("192.0.2.0/24", rpki.Prefix);
        Assert.Equal("213605", rpki.AsnValue);
    }

    [Fact]
    public void ClassifiesRpki_WithBareIpv4_AppendsSlash32()
    {
        var result = QueryAnalyzer.Analyze("192.0.2.1-213605-RPKI");
        var rpki = Assert.IsType<QueryType.Rpki>(result);
        Assert.Equal("192.0.2.1/32", rpki.Prefix);
    }

    [Fact]
    public void ClassifiesRpki_WithBareIpv6_AppendsSlash128()
    {
        var result = QueryAnalyzer.Analyze("2001:db8::1-213605-RPKI");
        var rpki = Assert.IsType<QueryType.Rpki>(result);
        Assert.Equal("2001:db8::1/128", rpki.Prefix);
    }

    [Fact]
    public void RpkiWithNonNumericAsn_IsUnknown_NotFallenThroughFurther()
    {
        var result = QueryAnalyzer.Analyze("192.0.2.0/24-notanumber-RPKI");
        Assert.IsType<QueryType.Unknown>(result);
    }

    [Fact]
    public void RpkiWithInvalidPrefix_IsUnknown()
    {
        var result = QueryAnalyzer.Analyze("not-a-prefix-213605-RPKI");
        Assert.IsType<QueryType.Unknown>(result);
    }

    [Fact]
    public void LongerSuffixIsTriedBeforeShorterOverlappingOne()
    {
        // A suffix ending in an internal dash before its own tail (e.g. "-FOO-BAR") literally
        // ends with a shorter registered suffix ("-BAR") as a substring — the one real case where
        // ends_with-based matching is ambiguous depending on check order (plan.md §4.2). None of
        // the suffixes actually shipped in Phase 1 have this property (verified: no registered
        // suffix is a literal string-tail of another), but the registry must still get this right
        // for suffixes added later that might, so the test constructs the ambiguity directly
        // rather than relying on a real suffix pair that happens not to exercise it.
        QueryAnalyzer.RegisterSuffix("-BAR", _ => new QueryType.Unknown("wrong-branch-matched"));
        QueryAnalyzer.RegisterSuffix("-FOO-BAR", baseQuery => new QueryType.Desc(baseQuery));

        var result = QueryAnalyzer.Analyze("example.com-FOO-BAR");
        var desc = Assert.IsType<QueryType.Desc>(result);
        Assert.Equal("example.com", desc.BaseQuery);
    }
}

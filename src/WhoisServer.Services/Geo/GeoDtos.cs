using System.Text.Json.Serialization;

namespace WhoisServer.Services.Geo;

// Ported from src/services/geo/types.rs — only the fields actually consumed by the formatters
// below are kept (the Rust DTOs carry several #[allow(dead_code)] fields never used for display).

public sealed record RipeStatResponse(
    [property: JsonPropertyName("data")] RipeStatData? Data,
    [property: JsonPropertyName("status")] string Status);

public sealed record RipeStatData(
    [property: JsonPropertyName("located_resources")] List<LocatedResource>? LocatedResources);

public sealed record LocatedResource(
    [property: JsonPropertyName("resource")] string Resource,
    [property: JsonPropertyName("locations")] List<GeoLocation>? Locations);

public sealed record GeoLocation(
    [property: JsonPropertyName("country")] string? Country,
    [property: JsonPropertyName("city")] string? City,
    [property: JsonPropertyName("latitude")] double? Latitude,
    [property: JsonPropertyName("longitude")] double? Longitude);

public sealed record IpinfoResponse(
    [property: JsonPropertyName("ip")] string Ip,
    [property: JsonPropertyName("asn")] string? Asn,
    [property: JsonPropertyName("as_name")] string? AsName,
    [property: JsonPropertyName("as_domain")] string? AsDomain,
    [property: JsonPropertyName("country")] string? Country,
    [property: JsonPropertyName("continent")] string? Continent,
    [property: JsonPropertyName("city")] string? City,
    [property: JsonPropertyName("region")] string? Region);

public sealed record IpApiResponse(
    [property: JsonPropertyName("query")] string Ip,
    [property: JsonPropertyName("status")] string Status,
    [property: JsonPropertyName("country")] string? Country,
    [property: JsonPropertyName("countryCode")] string? CountryCode,
    [property: JsonPropertyName("regionName")] string? RegionName,
    [property: JsonPropertyName("city")] string? City,
    [property: JsonPropertyName("lat")] double? Lat,
    [property: JsonPropertyName("lon")] double? Lon,
    [property: JsonPropertyName("timezone")] string? Timezone,
    [property: JsonPropertyName("isp")] string? Isp,
    [property: JsonPropertyName("org")] string? Org,
    [property: JsonPropertyName("as")] string? AsnInfo);

public sealed record BilibiliIpResponse(
    [property: JsonPropertyName("code")] int Code,
    [property: JsonPropertyName("message")] string Message,
    [property: JsonPropertyName("data")] BilibiliIpData? Data);

public sealed record BilibiliIpData(
    [property: JsonPropertyName("addr")] string Addr,
    [property: JsonPropertyName("country")] string Country,
    [property: JsonPropertyName("province")] string Province,
    [property: JsonPropertyName("city")] string City,
    [property: JsonPropertyName("isp")] string Isp);

public sealed record MeituanIpResponse([property: JsonPropertyName("data")] MeituanIpData? Data);

public sealed record MeituanIpData(
    [property: JsonPropertyName("lng")] double Lng,
    [property: JsonPropertyName("ip")] string Ip,
    [property: JsonPropertyName("rgeo")] MeituanRgeo Rgeo,
    [property: JsonPropertyName("lat")] double Lat);

public sealed record MeituanRgeo(
    [property: JsonPropertyName("country")] string Country,
    [property: JsonPropertyName("province")] string Province,
    [property: JsonPropertyName("city")] string City,
    [property: JsonPropertyName("district")] string District);

public sealed record RirGeoResponse(
    [property: JsonPropertyName("data")] RirGeoData? Data,
    [property: JsonPropertyName("status")] string Status);

public sealed record RirGeoData(
    [property: JsonPropertyName("located_resources")] List<RirGeoResource>? LocatedResources);

public sealed record RirGeoResource(
    [property: JsonPropertyName("resource")] string Resource,
    [property: JsonPropertyName("location")] string Location);

public sealed record PrefixesResponse(
    [property: JsonPropertyName("data")] PrefixesData? Data,
    [property: JsonPropertyName("status")] string Status);

public sealed record PrefixesData(
    [property: JsonPropertyName("prefixes")] List<PrefixInfo>? Prefixes);

public sealed record PrefixInfo([property: JsonPropertyName("prefix")] string Prefix);

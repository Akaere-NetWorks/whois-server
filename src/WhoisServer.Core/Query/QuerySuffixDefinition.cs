namespace WhoisServer.Core.Query;

/// <summary>
/// One registered query suffix (e.g. "-DESC"). <see cref="QueryAnalyzer"/> tries these in
/// descending suffix-length order before falling back to generic IP/ASN/domain sniffing — a
/// defensive default rather than a fix for an existing ambiguity (see plan.md §4.2): it only
/// matters for a suffix containing its own internal dash, e.g. a future "-FOO-BAR" literally ends
/// with the substring "-BAR", so "-BAR" must not be checked first or "-FOO-BAR" would never match.
/// </summary>
/// <param name="Suffix">Upper-case suffix including the leading dash, e.g. "-DESC".</param>
/// <param name="Factory">Builds the <see cref="QueryType"/> from the suffix-stripped base query
/// (original casing preserved, matching the Rust implementation's byte-slice-not-recase behavior).</param>
public sealed record QuerySuffixDefinition(string Suffix, Func<string, QueryType> Factory);

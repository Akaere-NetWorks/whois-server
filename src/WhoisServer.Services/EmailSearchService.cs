using WhoisServer.Core.Dn42;

namespace WhoisServer.Services;

/// <summary>
/// Ported from src/services/email.rs — the <c>-EMAIL</c> suffix. A gap from this port's earlier
/// phases (Phase 2's network-intelligence services never included it — found while implementing
/// Phase 7's full feature-flag matrix and cross-checking against Cargo.toml's atomic feature
/// list). Despite its generic-sounding name, this is a DN42-specific feature: it recursively
/// queries the DN42 registry for a person/mntner/organisation object, extracts any e-mail/
/// abuse-mailbox fields, follows <c>mnt-by</c>/<c>admin-c</c>/<c>tech-c</c> references to other
/// objects, and aggregates every email address found.
///
/// Optional <see cref="IDn42QueryRouter"/> dependency mirrors <c>ResolutionOrchestrator</c>'s
/// pattern: null whenever Feature_Dn42 is off, in which case this always returns a
/// "requires DN42" message rather than throwing.
/// </summary>
public sealed class EmailSearchService(IDn42QueryRouter? dn42Router = null)
{
    public async Task<string> ProcessAsync(string baseQuery, CancellationToken ct = default)
    {
        if (dn42Router is null)
        {
            return "% Email Search\n% This feature requires the DN42 registry (Feature_Dn42), which is not enabled in this build.\n";
        }

        var baseResponse = await dn42Router.ResolveAsync(baseQuery, ct);

        var emails = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var email in ExtractEmails(baseResponse))
        {
            emails.Add(email);
        }

        var references = ExtractReferences(baseResponse);

        if (references.Count == 0 && emails.Count == 0)
        {
            var upper = baseQuery.ToUpperInvariant();
            var relatedQueries = new List<string>();
            if (!upper.EndsWith("-MNT", StringComparison.Ordinal))
            {
                relatedQueries.Add($"{baseQuery}-MNT");
            }

            if (!upper.EndsWith("-DN42", StringComparison.Ordinal))
            {
                relatedQueries.Add($"{baseQuery}-DN42");
            }

            foreach (var relatedQuery in relatedQueries)
            {
                var relatedResponse = await dn42Router.ResolveAsync(relatedQuery, ct);
                foreach (var email in ExtractEmails(relatedResponse))
                {
                    emails.Add(email);
                }

                foreach (var refName in ExtractReferences(relatedResponse))
                {
                    if (references.Contains(refName))
                    {
                        continue;
                    }

                    var refResponse = await dn42Router.ResolveAsync(refName, ct);
                    foreach (var email in ExtractEmails(refResponse))
                    {
                        emails.Add(email);
                    }
                }
            }
        }

        foreach (var reference in references)
        {
            var refResponse = await dn42Router.ResolveAsync(reference, ct);
            foreach (var email in ExtractEmails(refResponse))
            {
                emails.Add(email);
            }
        }

        return FormatResponse(emails);
    }

    private static List<string> ExtractReferences(string response)
    {
        var references = new List<string>();
        var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);

        foreach (var rawLine in response.Split('\n'))
        {
            var line = rawLine.Trim();
            var value = ExtractFieldValue(line, "mnt-by")
                ?? ExtractFieldValue(line, "admin-c")
                ?? ExtractFieldValue(line, "tech-c");

            if (value is not null && seen.Add(value))
            {
                references.Add(value);
            }
        }

        return references;
    }

    private static List<string> ExtractEmails(string response)
    {
        var emails = new List<string>();

        foreach (var rawLine in response.Split('\n'))
        {
            var line = rawLine.Trim();

            if (ExtractFieldValue(line, "abuse-mailbox") is { } abuseMailbox)
            {
                emails.Add(abuseMailbox);
            }
            else if (ExtractFieldValue(line, "e-mail") is { } eMail)
            {
                emails.Add(eMail);
            }
            else if (ExtractFieldValue(line, "email") is { } email)
            {
                emails.Add(email);
            }
            else if (ExtractFieldValue(line, "abuse-c") is { } abuseC && abuseC.Contains('@'))
            {
                emails.Add(abuseC);
            }
        }

        return emails;
    }

    private static string? ExtractFieldValue(string line, string fieldName)
    {
        if (!line.StartsWith(fieldName, StringComparison.Ordinal))
        {
            return null;
        }

        var colonPos = line.IndexOf(':');
        if (colonPos < 0)
        {
            return null;
        }

        var value = line[(colonPos + 1)..].Trim();
        return value.Length > 0 ? value : null;
    }

    private static string FormatResponse(HashSet<string> emails)
    {
        if (emails.Count == 0)
        {
            return "% Email Search\n% No email addresses found\n";
        }

        var sb = new System.Text.StringBuilder("% Email Search\n");
        foreach (var email in emails)
        {
            sb.Append($"e-mail:             {email}\n");
        }

        return sb.ToString();
    }
}

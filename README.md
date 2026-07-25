# whois-server

[![CI](https://github.com/Akaere-NetWorks/whois-server/actions/workflows/ci.yml/badge.svg)](https://github.com/Akaere-NetWorks/whois-server/actions/workflows/ci.yml)
[![Container image](https://github.com/Akaere-NetWorks/whois-server/actions/workflows/docker.yml/badge.svg)](https://github.com/Akaere-NetWorks/whois-server/actions/workflows/docker.yml)
[![Licence: AGPL v3](https://img.shields.io/badge/licence-AGPL--3.0-blue.svg)](LICENSE)

A multi-protocol WHOIS daemon: RFC 3912 on TCP 43, a 60-suffix query dispatcher fanning out to ~46
upstream service integrations, DN42 registry support, and a web dashboard with a JSON API — in one
.NET 10 process.

> **Note** — this is the C#/.NET rewrite of the original Rust implementation. The SSH server and
> the Lua plugin system are **not** carried over; see [Differences from the Rust
> implementation](#differences-from-the-rust-implementation).

---

## Contents

- [whois-server](#whois-server)
  - [Contents](#contents)
  - [What it does](#what-it-does)
  - [Quick start](#quick-start)
  - [Docker](#docker)
  - [Configuration](#configuration)
    - [CLI flags](#cli-flags)
    - [Environment variables](#environment-variables)
    - [State on disk](#state-on-disk)
  - [Query reference](#query-reference)
    - [Colour protocol](#colour-protocol)
  - [Web dashboard and REST API](#web-dashboard-and-rest-api)
  - [DN42](#dn42)
  - [Response patches](#response-patches)
  - [Plugins](#plugins)
  - [Feature flags and cut builds](#feature-flags-and-cut-builds)
  - [Development](#development)
  - [Project layout](#project-layout)
  - [Differences from the Rust implementation](#differences-from-the-rust-implementation)
  - [Licence](#licence)

---

## What it does

| | |
| --- | --- |
| **WHOIS server** | RFC 3912, TCP 43. Domains, IPv4/IPv6, CIDR, ASNs — resolved via IANA referral by default, RDAP optionally. |
| **Suffix dispatcher** | `example.com-DNS`, `AS213605-PEERINGDB`, `rust-CARGO`, … 60 built-in suffixes across five categories in a default build. |
| **DN42** | Git-mirrored private-internet registry, with automatic routing of DN42 IP/ASN ranges. |
| **Web dashboard + API** | Kestrel on port 9999: query UI, live stats, REST/JSON endpoints, OpenAPI document. |
| **Response patches** | SHA1-verified, remotely updatable DSL that rewrites specific lines of outgoing WHOIS text. |
| **Colour protocol** | Opt-in ANSI colourisation negotiated with an `X-WHOIS-COLOR:` header — an extension to raw WHOIS, invisible to standard clients. |
| **Native plugins** | Third-party query suffixes as ordinary .NET assemblies, loaded into an isolated `AssemblyLoadContext` with a manifest-declared permission set. |
| **Cut builds** | 44 MSBuild feature flags that genuinely remove code and dependencies, not just disable them at runtime. |

## Quick start

Requires the [.NET 10 SDK](https://dotnet.microsoft.com/download/dotnet/10.0). `git` must be on
`PATH` if DN42 is enabled.

```bash
git clone https://github.com/Akaere-NetWorks/whois-server.git
cd whois-server

# Port 4343 rather than 43 so it needs no root
make run                       # or: dotnet run --project src/WhoisServer.Host -- --port 4343
```

Then query it:

```bash
whois -h 127.0.0.1 -p 4343 example.com
whois -h 127.0.0.1 -p 4343 AS213605-PEERINGDB
whois -h 127.0.0.1 -p 4343 HELP           # authoritative suffix list for *your* build
```

…and open <http://127.0.0.1:9999> for the dashboard.

`HELP` is worth repeating: because suffixes are registered by whichever features were compiled in,
the running server's own help output is always the accurate list for that binary, where this README
can only describe a default build.

## Docker

Images are published to GHCR for `linux/amd64` and `linux/arm64`:

```bash
docker run -d --name whois-server \
  -p 43:43 -p 9999:9999 \
  -v whois-cache:/app/cache \
  ghcr.io/akaere-networks/whois-server:latest
```

Configuration is by environment variable and CLI flag — flags go after the image name, since they
are passed straight to the entrypoint:

```bash
docker run -d --name whois-server \
  -p 43:43 -p 9999:9999 \
  -v whois-cache:/app/cache \
  -v "$PWD/plugins:/app/plugins" \
  --env-file .env \
  ghcr.io/akaere-networks/whois-server:latest \
  --debug --max-connections 200
```

Notes:

- **`/app/cache` should be a volume.** Every SQLite store lives there (stats, patches, the DN42
  clone, referral caches). Without it, each restart re-clones the DN42 registry and discards stats.
- **The container runs as root** because binding port 43 is privileged. To run rootless, listen high
  inside the container and remap on the host:
  ```bash
  docker run -d -u 1654:1654 -p 43:4343 -p 9999:9999 -v whois-cache:/app/cache \
    ghcr.io/akaere-networks/whois-server:latest --port 4343
  ```
- **Building with feature flags** is supported through a build argument:
  ```bash
  docker build --build-arg FEATURES="-p:Feature_Dn42=false -p:Feature_Pixiv=false" -t whois-server .
  ```
  Disabling DN42 also makes the image's `git` dependency redundant.

## Configuration

### CLI flags

| Flag | Default | Meaning |
| --- | --- | --- |
| `-H`, `--host <HOST>` | `0.0.0.0` | Listen address, for **both** the WHOIS and web listeners |
| `-p`, `--port <PORT>` | `43` | WHOIS TCP port |
| `--web-port <PORT>` | `9999` | Dashboard / REST API port |
| `-d`, `--debug` | off | Debug-level logging |
| `-t`, `--trace` | off | Trace-level logging (extremely verbose) |
| `--max-connections <N>` | `100` | Concurrent WHOIS connection ceiling |
| `--timeout <SECONDS>` | `10` | Per-connection timeout |
| `--dump-traffic` | off | Write raw queries and responses to disk |
| `--dump-dir <DIR>` | `dumps` | Where `--dump-traffic` writes |
| `--enable-color` | `true` | Honour the `X-WHOIS-COLOR:` negotiation header |
| `-h`, `--help` | | Print usage |

### Environment variables

All are optional. Each unlocks one integration; when unset, that integration degrades (usually to
an explanatory WHOIS response) rather than failing startup.

| Variable | Used by |
| --- | --- |
| `IPINFO_TOKEN` | `-GEO` aggregated geolocation (ipinfo.io) |
| `IPINFO_API_TOKEN` | Hop geolocation in `-PING` / `-TRACEROUTE` |
| `STEAM_API_KEY` | `-STEAM` user profiles (app lookups work without it) |
| `OMDB_API_KEY` | `-IMDB` / `-IMDBSEARCH` |
| `CURSEFORGE_API_KEY` | `-CURSEFORGE` |
| `PIXIV_REFRESH_TOKEN` | `-PIXIV` and the image reverse-proxy |
| `TELEMETRY_ENABLED` | Set to `true` to enable query telemetry export |
| `TELEMETRY_ENDPOINT` | Telemetry collector URL (required when enabled) |
| `TELEMETRY_BEARER_TOKEN` | Telemetry `Authorization` bearer token |

> **The host does not read a `.env` file.** It reads real process environment variables only. Use
> your shell, systemd's `EnvironmentFile=`, or `docker run --env-file`. A stray `.env` in the
> working directory has no effect. (The one exception is `plugins/.plugins.env`, which the plugin
> loader reads on plugins' behalf — see [Plugins](#plugins).)

### State on disk

Everything is relative to the process working directory:

| Path | Contents |
| --- | --- |
| `cache/patches.db` | Loaded response-patch set |
| `cache/stats.db` | Query counters and byte totals |
| `cache/iana-referral.db` | Cached IANA → registry WHOIS referrals |
| `cache/dn42.db`, `cache/dn42-registry/` | DN42 object index and the git clone itself |
| `cache/manrs.db`, `peeringdb.db`, `icp.db`, `pen.db`, `openwrt.db` | Per-service caches |
| `cache/plugins.db` | Plugin-visible key/value store, namespaced per plugin |
| `plugins/` | Runtime-scanned plugin directory |
| `dumps/` | Only with `--dump-traffic` |

## Query reference

Suffixes are case-insensitive and matched longest-first, so a short suffix can never shadow a
longer one. A query with no recognised suffix is classified by shape (domain / IPv4 / IPv6 / CIDR /
ASN) and resolved normally.

**Core**

| Query | Result |
| --- | --- |
| `HELP` | Suffix list for this build |
| `UPDATE-PATCH` | Re-fetch response patches from the remote repository |
| `<query>-DESC` | Description/remarks-only summary |
| `<query>-RDAP` | Force an RDAP lookup (works regardless of `Feature_RdapBackend`) |

**Network intelligence**

| Query | Result |
| --- | --- |
| `<AS/prefix>-BGPTOOL` | BGP looking-glass data via bgp.tools |
| `<resource>-IRR` | IRR Explorer route-object lookup |
| `<resource>-LG` | Looking-glass query |
| `AS<n>-MANRS` | MANRS compliance status |
| `AS<n>-PEERINGDB` | PeeringDB network record |
| `AS<n>-PREFIXES` | Announced prefixes |
| `<ip>-GEO` | Aggregated geolocation, multiple sources |
| `<ip>-RIRGEO` | RIR-reported geolocation |
| `<domain>-DNS` | DNS records over DoH |
| `<host>-PING` | Globalping ping measurement |
| `<host>-TRACEROUTE`, `<host>-TRACE` | Globalping traceroute (`-TRACE` is an alias) |
| `<server>-NTP` | NTP server query |
| `<domain>-SSL` | TLS certificate details |
| `<domain>-CRT` | crt.sh certificate-transparency search |
| `CFSTATUS` | Cloudflare status summary |
| `<resource>-EMAIL` | DN42 e-mail search |
| `<resource>-<REGISTRY>` | Direct query to one IRR registry |

The direct-registry suffixes are `-RADB`, `-ALTDB`, `-AFRINIC`, `-APNIC`, `-ARIN`, `-BELL`,
`-JPIRR`, `-LACNIC`, `-LEVEL3`, `-NTTCOM`, `-RIPE`, `-RIS` and `-TC`.

**Miscellaneous**

`<domain>-ICP` (China ICP filing) · `<n|name>-PEN` (IANA Private Enterprise Number) ·
`<id>-PIXIV` (artwork metadata)

**Entertainment**

`-STEAM` / `-STEAMSEARCH` · `-IMDB` / `-IMDBSEARCH` · `-MINECRAFT` (`-MC`) · `-MCU`
(Minecraft user) · `-ACGC` (characters) · `-LYRIC` · `-WIKIPEDIA` · `-GITHUB`

Random meal suggestions are matched by whole query as well as by suffix: `今天吃什么` or
`<x>-MEAL` (TheMealDB), and `今天吃什么中国` or `<x>-MEAL-CN` (a bundled Chinese recipe dataset).

**Package registries**

`-ALMA` · `-AOSC` · `-AUR` · `-CARGO` · `-CURSEFORGE` · `-DEBIAN` · `-EPEL` · `-MODRINTH` ·
`-NIXOS` · `-NPM` · `-OPENSUSE` · `-OPENWRT` · `-PYPI` · `-UBUNTU`

### Colour protocol

An extension, not part of RFC 3912. A client may send a header line before its query:

```
X-WHOIS-COLOR-PROBE: 1.0        →  X-WHOIS-COLOR-SUPPORT: 1.0 schemes=ripe,ripe-dark,bgptools,bgptools-dark
X-WHOIS-COLOR: ripe-dark        →  the following response is ANSI-colourised
```

Standard clients never send these headers and so never see colour. Disable server-side with
`--enable-color=false`.

## Web dashboard and REST API

Served by Kestrel on `--web-port` (default 9999).

| Endpoint | Purpose |
| --- | --- |
| `GET /` | Dashboard (query UI + statistics) |
| `GET /api/whois?q=<query>` | Run a query, JSON response |
| `POST /api/whois` | Same, query in the JSON body |
| `GET /raw/{query}` | Run a query, `text/plain` response |
| `GET /api/stats` | Request counters, bytes served, 24-hour histogram |
| `GET /api/openapi.json` | OpenAPI document |
| `GET /pixiv-proxy/{**path}` | Pixiv image reverse-proxy (needs `Feature_Pixiv`) |
| `GET /pixiv-proxy-health` | Proxy liveness check |

```bash
curl 'http://127.0.0.1:9999/raw/example.com'
curl 'http://127.0.0.1:9999/api/whois?q=AS213605-PEERINGDB'
curl -X POST http://127.0.0.1:9999/api/whois -H 'content-type: application/json' -d '{"query":"1.1.1.1-GEO"}'
```

> `Feature_Web=false` removes the dashboard, the API routes and static-file serving, but Kestrel
> still **binds** the web port and answers 404 — the TCP listener and the HTTP host are deliberately
> one process. If the port must not be bound at all, keep it on loopback with `--host`.

## DN42

With `Feature_Dn42` on (the default), the server shallow-clones
`https://git.akae.re/pysio/mirrors-dn42.git` into `cache/dn42-registry/` at startup and pulls
hourly, indexing objects into `cache/dn42.db`.

Queries falling inside DN42's private ranges are routed to that registry automatically, so
`172.20.0.1` or `AS4242420000` need no suffix. Handles ending in `-DN42` or `-MNT` (person and
maintainer objects) are also recognised directly.

Sync happens through the system `git` binary, so `git` must be installed. A failed sync is logged
and retried on the next tick; it never stops the server.

## Response patches

A small DSL rewrites specific lines of outgoing WHOIS text — used to correct or annotate upstream
data. Patches load from `cache/patches.db` at startup; to fetch or refresh them, send the
`UPDATE-PATCH` query. Failure to load patches is a warning, not a fatal error: the server starts and
serves unpatched text.

The patch set is fetched over HTTPS from a `patches.json` manifest and **SHA1-verified per patch**
against the digest that manifest declares, so a tampered or truncated download is rejected rather
than applied. The manifest URL points at [`patches/patches.json`](patches/) on this repository's
`main` branch — that directory is the published source of truth for every deployed server, not just
a local copy, so **removing it breaks patch updates everywhere**. The URL is a constant in
`PatchManager`, with no CLI flag or environment override: running a private patch set means changing
it in source.

To add a patch: drop the `.patch` file in `patches/`, run `patches/update-patches-json.sh` to
recompute the manifest's digests and sizes, and verify with `patches/verify-patches.sh`. See
[`patches/README.md`](patches/README.md) for the DSL itself.

## Plugins

A plugin is an ordinary .NET assembly plus a `plugin.json` manifest, in its own directory under
`plugins/`. Each is loaded into an isolated `AssemblyLoadContext` and given only what its manifest
requests.

```
plugins/
├── .plugins.env          # optional KEY=VALUE lines, exposed per manifest `envVars`
└── weather/
    ├── plugin.json
    └── Weather.dll
```

```jsonc
{
  "name": "weather",
  "version": "1.0.0",
  "suffix": "-WEATHER",              // rejected at load time if already claimed
  "author": "Akaere Networks",
  "description": "Weather information via wttr.in",
  "enabled": true,
  "timeoutSeconds": 5,               // exceeded → the query fails, the server does not
  "entryPoint": "Weather.dll",
  "permissions": {
    "network": true,
    "allowedDomains": ["wttr.in"],   // enforced by an HTTP handler, not advisory
    "cacheRead": true,
    "cacheWrite": true,              // a namespaced slice of cache/plugins.db
    "envVars": []
  }
}
```

The contract is one method:

```csharp
public sealed class WeatherPlugin(HttpClient http) : IWhoisPlugin
{
    // `query` arrives already suffix-stripped
    public Task<string> ProcessAsync(string query, CancellationToken ct = default) => /* … */;
}
```

The type must have **exactly one** public constructor. Its parameters may include an `HttpClient`
(wrapped in the domain whitelist) and an `IKvStore` (namespaced to this plugin); the loader supplies
whichever are present, and refuses to load the plugin if a parameter asks for something the manifest
does not permit.

`plugins-src/Weather/` is a complete worked example. Building it copies the DLL and manifest into
`plugins/weather/` automatically:

```bash
dotnet build plugins-src/Weather/Weather.csproj
```

When writing your own, reference `WhoisServer.Plugins` and `WhoisServer.Core` with
`Private="false" ExcludeAssets="runtime"` — copying those assemblies next to your plugin would load
second copies of the shared contract types into your isolated context and break the
`IWhoisPlugin` type check at runtime. `plugins-src/Weather/Weather.csproj` documents this in detail.

## Feature flags and cut builds

44 MSBuild properties, one per atomic feature, each defaulting to `true` — except
`Feature_RdapBackend`, which defaults to `false` so the default resolution order stays
WHOIS-via-IANA-referral rather than RDAP-first.

A flag does two things at once, closely mirroring Cargo's `#[cfg(feature = "…")]`:

1. Drops that feature's source files from compilation (`<Compile Remove>`), and
2. undefines its `FEATURE_*` preprocessor symbol, so `#if`-guarded registrations and dispatch cases
   disappear too.

The result is a genuinely smaller binary with the disabled dependencies absent, not a build with
dead code switched off at runtime.

```bash
dotnet build whois-server.slnx -p:Feature_Dn42=false
dotnet publish src/WhoisServer.Host -c Release -p:Feature_Web=false -p:Feature_Pixiv=false
make minimal                                    # DN42 excluded
```

`Feature_Packages` is a convenience preset, not an atomic flag: the 14 `Feature_Pkg*` flags default
to whatever it resolved to, so `-p:Feature_Packages=false` cascades to all of them while
`-p:Feature_PkgNpm=false` still disables just npm.

The full list lives in [`Directory.Build.props`](Directory.Build.props), which is the single source
of truth. CI builds several combinations plus an every-flag-off build whose flag list is derived
from that file, so a newly added flag is covered without editing the workflow.

## Development

```bash
make help              # list targets
make build             # Debug build of the solution
make run               # run on port 4343
make watch             # dotnet watch, auto-restart
make test              # everything, including live-network tests
make test-unit         # Core tests only — no network needed
make test-integration  # live WHOIS/RDAP/HTTP tests
make format            # apply style fixes
make format-check      # verify style, CI-equivalent
make publish           # framework-dependent Release build into publish/
make clean             # remove bin/obj, cache/ and publish/
```

Three test projects, 61 tests:

- **`WhoisServer.Core.Tests`** (45) — query classification, patch DSL, colouriser, storage. Offline.
- **`WhoisServer.Plugins.Tests`** (10) — manifest parsing, registry collisions, end-to-end load of a
  real compiled plugin.
- **`WhoisServer.Integration.Tests`** (6) — boots the actual DI-wired host on a free port and drives
  it over TCP, resolving real domains against real registries. Needs outbound 43 and 443, and is
  non-blocking in CI for that reason.

CI (`.github/workflows/ci.yml`) gates on a warning-free Release build, the offline tests,
`dotnet format --verify-no-changes`, and the cut-build matrix. Images are built and pushed by
`docker.yml`; tagged releases produce self-contained tarballs via `release.yml`.

## Project layout

```
src/
├── WhoisServer.Core/            # query classification, patch DSL, colour protocol, SQLite stores, stats
├── WhoisServer.Services/        # ~70 upstream integrations (BGP, geo, DNS, packages, entertainment, …)
├── WhoisServer.Services.Dn42/   # DN42 git mirror, object index, private-range routing
├── WhoisServer.Plugins/         # plugin loader, isolated load contexts, permission enforcement
└── WhoisServer.Host/            # composition root: TCP 43 listener, Kestrel, dashboard, CLI
plugins-src/Weather/             # worked plugin example
tests/                           # Core, Plugins, Integration
```

Storage is SQLite throughout (`Microsoft.Data.Sqlite`) behind a narrow `IKvStore`, one database file
per subsystem.

## Differences from the Rust implementation

Deliberate, locked-in decisions of the rewrite — not gaps awaiting work:

| | |
| --- | --- |
| **SSH server dropped** | The interactive shell on port 2222 is gone entirely. WHOIS over TCP 43 and the web API are the only front doors. |
| **Lua plugins replaced** | No embedded scripting engine. Extensibility is native .NET assemblies with manifest-declared permissions (see [Plugins](#plugins)). |
| **DN42 backend unified** | One git-backed, cross-platform implementation. The Windows / online-HTTP-fetch variant is gone. |
| **SQLite instead of LMDB** | Same `IKvStore` shape, one file per subsystem. |
| **RDAP stays opt-in** | Matching upstream's actual default (its `full` preset omits `rdap-backend`), though the explicit `-RDAP` suffix now works in every build rather than requiring a recompile. |

Not a byte-for-byte port: correctness and idiomatic .NET design took precedence, and known upstream
bugs were fixed in passing rather than reproduced.

## Licence

GNU Affero General Public License v3.0 or later. See [LICENSE](LICENSE) and [COPYRIGHT](COPYRIGHT).

Issues and questions: <noc@akae.re> or the
[issue tracker](https://github.com/Akaere-NetWorks/whois-server/issues).

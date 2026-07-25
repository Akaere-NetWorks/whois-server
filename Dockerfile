# syntax=docker/dockerfile:1

# ---------------------------------------------------------------------------
# Stage 1: publish
#
# Pinned to the BUILD platform, not the target one: the publish below passes
# `-a $TARGETARCH`, so a single native-speed SDK container cross-publishes for
# linux/amd64 and linux/arm64 instead of running the whole SDK under QEMU.
# ---------------------------------------------------------------------------
FROM --platform=$BUILDPLATFORM mcr.microsoft.com/dotnet/sdk:10.0 AS build

ARG TARGETARCH
ARG BUILD_CONFIGURATION=Release

# Cut-build flags, forwarded verbatim to restore and publish. Deliberately left
# unquoted at the point of use so multiple flags word-split into separate
# arguments, e.g.:
#   docker build --build-arg FEATURES="-p:Feature_Dn42=false -p:Feature_Web=false" .
# See "Feature flags" in README.md for the full list.
ARG FEATURES=""

WORKDIR /src

# Project files only, restored on their own, so the slow NuGet layer stays
# cached until a .csproj or Directory.Build.props actually changes. The props
# file carries every Feature_* default and so must land before the restore.
COPY Directory.Build.props ./
COPY src/WhoisServer.Core/WhoisServer.Core.csproj                   src/WhoisServer.Core/
COPY src/WhoisServer.Services/WhoisServer.Services.csproj           src/WhoisServer.Services/
COPY src/WhoisServer.Services.Dn42/WhoisServer.Services.Dn42.csproj src/WhoisServer.Services.Dn42/
COPY src/WhoisServer.Plugins/WhoisServer.Plugins.csproj             src/WhoisServer.Plugins/
COPY src/WhoisServer.Host/WhoisServer.Host.csproj                   src/WhoisServer.Host/
RUN dotnet restore src/WhoisServer.Host/WhoisServer.Host.csproj -a "$TARGETARCH" $FEATURES

COPY src/ src/

# Framework-dependent (--no-self-contained): the runtime stage is already an
# ASP.NET Core image, so bundling a second copy of the runtime would roughly
# double the image for nothing. Release tarballs are the self-contained ones.
RUN dotnet publish src/WhoisServer.Host/WhoisServer.Host.csproj \
        -c "$BUILD_CONFIGURATION" \
        -a "$TARGETARCH" \
        --no-self-contained \
        --no-restore \
        $FEATURES \
        -o /app/publish

# ---------------------------------------------------------------------------
# Stage 2: runtime
# ---------------------------------------------------------------------------
FROM mcr.microsoft.com/dotnet/aspnet:10.0 AS runtime

LABEL org.opencontainers.image.title="whois-server" \
      org.opencontainers.image.description="Multi-protocol WHOIS server with DN42 support, 45+ query suffixes, and a web dashboard" \
      org.opencontainers.image.source="https://github.com/Akaere-NetWorks/whois-server" \
      org.opencontainers.image.licenses="AGPL-3.0-or-later"

# `git` is a hard runtime dependency, not a convenience: the DN42 backend
# shells out to the system git binary for `clone --depth 1` / `pull` rather
# than linking a native git library (see WhoisServer.Services.Dn42's .csproj
# comment). Without it, DN42 sync logs a failure every hour and DN42 queries
# return nothing — everything else still works. Drop this layer only if you
# also build with -p:Feature_Dn42=false.
RUN apt-get update \
    && apt-get install -y --no-install-recommends git ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=build /app/publish ./

# This image redistributes AGPL-3.0 software; ship its licence text with it.
COPY LICENSE COPYRIGHT /app/

# Every SqliteKvStore (patches, stats, DN42 mirror, IANA referral cache,
# MANRS/PeeringDB/ICP/PEN/OpenWrt caches) lands under ./cache, resolved
# relative to the working directory — Program.cs passes "./cache" literally.
# Keep it on a volume so a restart doesn't re-clone the DN42 registry or lose
# accumulated query stats.
VOLUME ["/app/cache"]

# Runtime-scanned plugin directory (see "Plugins" in README.md). Empty in this
# image by design — plugins are operator-supplied; bind-mount your own.
RUN mkdir -p /app/plugins

# 43 = WHOIS (RFC 3912), 9999 = web dashboard + REST API.
EXPOSE 43/tcp
EXPOSE 9999/tcp

# Intentionally root. The app binds port 43 directly, which is privileged, so a
# non-root USER here would fail to start unless the operator remembers to grant
# CAP_NET_BIND_SERVICE. Container root is namespaced and this process opens no
# host paths beyond /app, so the trade is a deliberate one — README.md documents
# the rootless recipe (run on a high port, publish it as 43 on the host).
ENTRYPOINT ["./WhoisServer.Host"]

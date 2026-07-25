#!/usr/bin/env bash
#
# build-deb.sh — assemble a binary .deb for whois-server.
#
# Deliberately plain dpkg-deb rather than debhelper: the payload is a single
# `dotnet publish` output tree, there is nothing to compile in the Debian sense,
# and this way an amd64 CI runner can emit the arm64 package without an sbuild
# chroot or QEMU. The filesystem layout itself lives in
# packaging/common/stage-tree.sh, shared with the .rpm build.
#
# Usage:
#   packaging/deb/build-deb.sh [options]
#
#   --rid <RID>            .NET runtime identifier    [default: linux-x64]
#   --version <VERSION>    Package version            [default: <VersionPrefix> from Directory.Build.props]
#   --output <DIR>         Where the .deb is written  [default: dist]
#   --publish-dir <DIR>    Use an existing publish tree instead of running dotnet publish
#   --features "<FLAGS>"   MSBuild cut-build flags, e.g. '-p:Feature_Dn42=false'
#   --no-plugin            Skip building and shipping the Weather example plugin
#
# Examples:
#   packaging/deb/build-deb.sh --version 1.4.0
#   packaging/deb/build-deb.sh --rid linux-arm64 --features '-p:Feature_Dn42=false'

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DEB_DIR="$REPO_ROOT/packaging/deb"
# shellcheck source=../common/stage-tree.sh
. "$REPO_ROOT/packaging/common/stage-tree.sh"

RID=linux-x64
VERSION=
OUTPUT_DIR=
PUBLISH_DIR=
FEATURES=
WITH_PLUGIN=1

die() { echo "build-deb: $*" >&2; exit 1; }

while [ $# -gt 0 ]; do
    case "$1" in
        --rid)         RID="${2:?--rid needs a value}"; shift 2 ;;
        --version)     VERSION="${2:?--version needs a value}"; shift 2 ;;
        --output)      OUTPUT_DIR="${2:?--output needs a value}"; shift 2 ;;
        --publish-dir) PUBLISH_DIR="${2:?--publish-dir needs a value}"; shift 2 ;;
        --features)    FEATURES="${2-}"; shift 2 ;;
        --no-plugin)   WITH_PLUGIN=0; shift ;;
        -h|--help)     sed -n '3,25p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *)             die "unknown option: $1" ;;
    esac
done

OUTPUT_DIR="${OUTPUT_DIR:-$REPO_ROOT/dist}"

command -v dpkg-deb >/dev/null || die "dpkg-deb not found (apt-get install dpkg-dev)"

# dpkg architecture names do not match .NET RIDs, and the mapping is the one
# thing that must be right for apt to consider the package installable.
case "$RID" in
    linux-x64)     DEB_ARCH=amd64 ;;
    linux-arm64)   DEB_ARCH=arm64 ;;
    linux-arm)     DEB_ARCH=armhf ;;
    linux-musl-*)  die "musl RIDs produce packages Debian/Ubuntu cannot use: $RID" ;;
    *)             die "no dpkg architecture known for RID: $RID" ;;
esac

VERSION="$(resolve_version "$VERSION")" || die "could not resolve a package version"

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

echo "==> whois-server $VERSION  ($RID -> $DEB_ARCH)"

# ---------------------------------------------------------------------------
# 1. Application payload
# ---------------------------------------------------------------------------
if [ -z "$PUBLISH_DIR" ]; then
    PUBLISH_DIR="$STAGE/publish"
    publish_host "$RID" "$VERSION" "$FEATURES" "$PUBLISH_DIR"
else
    [ -d "$PUBLISH_DIR" ] || die "--publish-dir does not exist: $PUBLISH_DIR"
    echo "==> reusing publish tree at $PUBLISH_DIR"
fi

[ -x "$PUBLISH_DIR/WhoisServer.Host" ] || die "no WhoisServer.Host executable in $PUBLISH_DIR"

# ---------------------------------------------------------------------------
# 2. Filesystem layout (shared with the .rpm build)
# ---------------------------------------------------------------------------
ROOT="$STAGE/root"
ENV_CONF="etc/default/$PKG_NAME"
stage_tree "$PUBLISH_DIR" "$ROOT" "$ENV_CONF" "$WITH_PLUGIN"

# Debian wants a machine-readable copyright file; RPM covers the same ground
# with the spec's License: field, so this one file is deb-only.
install -m 0644 "$DEB_DIR/copyright" "$ROOT/usr/share/doc/$PKG_NAME/copyright"

# ---------------------------------------------------------------------------
# 3. Control metadata
# ---------------------------------------------------------------------------
CONTROL="$ROOT/DEBIAN"
install -d -m 0755 "$CONTROL"

INSTALLED_SIZE="$(du -sk --exclude=DEBIAN "$ROOT" | cut -f1)"

# libicu is versioned differently in every Debian and Ubuntu release and there
# is no virtual package to depend on, so the alternatives list is the only way
# to stay installable from Debian 11 through Ubuntu 24.04+. Without it the
# runtime aborts at startup unless DOTNET_SYSTEM_GLOBALIZATION_INVARIANT is set.
# git is a hard dependency rather than a recommendation because the DN42 backend
# shells out to the binary; build with -p:Feature_Dn42=false if that is unwanted.
cat > "$CONTROL/control" <<EOF
Package: $PKG_NAME
Version: $VERSION
Architecture: $DEB_ARCH
Maintainer: Akaere Networks <noc@akae.re>
Installed-Size: $INSTALLED_SIZE
Depends: libc6, libgcc-s1, libstdc++6, zlib1g, ca-certificates,
 libssl3t64 | libssl3 | libssl1.1,
 libicu76 | libicu74 | libicu72 | libicu71 | libicu70 | libicu69 | libicu67,
 git, adduser
Section: net
Priority: optional
Homepage: https://github.com/Akaere-NetWorks/whois-server
Description: Multi-protocol WHOIS server with DN42 support and a web dashboard
 A WHOIS daemon speaking RFC 3912 on TCP port 43, with a 60-suffix query
 dispatcher that fans out to around 46 upstream service integrations (BGP,
 geolocation, DNS, package registries and more), first-class DN42 registry
 support backed by a local git mirror, and a web dashboard with a JSON API.
 .
 Runs as a single .NET 10 process under systemd as an unprivileged service
 account, using CAP_NET_BIND_SERVICE to bind the privileged WHOIS port. The
 .NET runtime is bundled, so no system dotnet installation is required.
EOF

echo "/$ENV_CONF" > "$CONTROL/conffiles"

for script in postinst prerm postrm; do
    install -m 0755 "$DEB_DIR/$script" "$CONTROL/$script"
done

# md5sums covers package-managed files only: conffiles are excluded because dpkg
# tracks them separately, and DEBIAN/ is not part of the installed filesystem.
( cd "$ROOT" && find . -type f ! -path './DEBIAN/*' ! -path './etc/*' -printf '%P\0' \
    | sort -z | xargs -0 -r md5sum > "$CONTROL/md5sums" )
chmod 0644 "$CONTROL/md5sums"

# ---------------------------------------------------------------------------
# 4. Build
# ---------------------------------------------------------------------------
mkdir -p "$OUTPUT_DIR"
DEB_PATH="$OUTPUT_DIR/${PKG_NAME}_${VERSION}_${DEB_ARCH}.deb"

# --root-owner-group so the package does not record the CI runner's uid; xz is
# worth the build time here because the payload is a ~90 MB runtime tree.
dpkg-deb --build --root-owner-group -Zxz "$ROOT" "$DEB_PATH"

( cd "$OUTPUT_DIR" && sha256sum "$(basename "$DEB_PATH")" > "$(basename "$DEB_PATH").sha256" )

echo "==> $DEB_PATH"
dpkg-deb --info "$DEB_PATH" | sed 's/^/    /'

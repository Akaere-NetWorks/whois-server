#!/usr/bin/env bash
#
# build-rpm.sh — assemble a binary .rpm for whois-server.
#
# Mirrors packaging/deb/build-deb.sh: the installed filesystem comes from the
# shared stage_tree() in packaging/common/stage-tree.sh, so the .deb and the .rpm
# lay their files out identically and share one systemd unit. The only work left
# is tarring that tree up as the spec's Source0 and calling rpmbuild.
#
# Cross-building works because nothing is compiled under rpmbuild — `dotnet
# publish -r linux-arm64` produces the payload and `rpmbuild --target aarch64`
# just labels it. No mock chroot, no QEMU.
#
# Usage:
#   packaging/rpm/build-rpm.sh [options]
#
#   --rid <RID>            .NET runtime identifier    [default: linux-x64]
#   --version <VERSION>    Package version            [default: <VersionPrefix> from Directory.Build.props]
#   --release <N>          RPM release field          [default: 1]
#   --output <DIR>         Where the .rpm is written  [default: dist]
#   --publish-dir <DIR>    Use an existing publish tree instead of running dotnet publish
#   --features "<FLAGS>"   MSBuild cut-build flags, e.g. '-p:Feature_Dn42=false'
#   --no-plugin            Skip building and shipping the Weather example plugin
#
# Examples:
#   packaging/rpm/build-rpm.sh --version 1.4.0
#   packaging/rpm/build-rpm.sh --rid linux-arm64 --features '-p:Feature_Dn42=false'

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RPM_DIR="$REPO_ROOT/packaging/rpm"
# shellcheck source=../common/stage-tree.sh
. "$REPO_ROOT/packaging/common/stage-tree.sh"

RID=linux-x64
VERSION=
RPM_RELEASE=1
OUTPUT_DIR=
PUBLISH_DIR=
FEATURES=
WITH_PLUGIN=1

die() { echo "build-rpm: $*" >&2; exit 1; }

while [ $# -gt 0 ]; do
    case "$1" in
        --rid)         RID="${2:?--rid needs a value}"; shift 2 ;;
        --version)     VERSION="${2:?--version needs a value}"; shift 2 ;;
        --release)     RPM_RELEASE="${2:?--release needs a value}"; shift 2 ;;
        --output)      OUTPUT_DIR="${2:?--output needs a value}"; shift 2 ;;
        --publish-dir) PUBLISH_DIR="${2:?--publish-dir needs a value}"; shift 2 ;;
        --features)    FEATURES="${2-}"; shift 2 ;;
        --no-plugin)   WITH_PLUGIN=0; shift ;;
        -h|--help)     sed -n '3,28p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *)             die "unknown option: $1" ;;
    esac
done

OUTPUT_DIR="${OUTPUT_DIR:-$REPO_ROOT/dist}"

command -v rpmbuild >/dev/null || \
    die "rpmbuild not found (dnf install rpm-build, or apt-get install rpm)"

# RPM architecture names match neither .NET RIDs nor dpkg's. ExclusiveArch in
# the spec covers the same two, so keep the lists in step.
case "$RID" in
    linux-x64)    RPM_ARCH=x86_64 ;;
    linux-arm64)  RPM_ARCH=aarch64 ;;
    linux-musl-*) die "musl RIDs produce packages Fedora/RHEL cannot use: $RID" ;;
    *)            die "no rpm architecture known for RID: $RID" ;;
esac

VERSION="$(resolve_version "$VERSION")" || die "could not resolve a package version"

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

echo "==> whois-server $VERSION-$RPM_RELEASE  ($RID -> $RPM_ARCH)"

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
# 2. Filesystem layout (shared with the .deb build)
# ---------------------------------------------------------------------------
ROOT="$STAGE/root"
stage_tree "$PUBLISH_DIR" "$ROOT" "etc/sysconfig/$PKG_NAME" "$WITH_PLUGIN"

# ---------------------------------------------------------------------------
# 3. rpmbuild
# ---------------------------------------------------------------------------
# A private _topdir so the build never touches ~/rpmbuild and concurrent matrix
# legs cannot collide over SOURCES/.
TOPDIR="$STAGE/rpmbuild"
mkdir -p "$TOPDIR"/{SOURCES,SPECS,BUILD,BUILDROOT,RPMS,SRPMS}

PAYLOAD="$TOPDIR/SOURCES/${PKG_NAME}-payload-${VERSION}-${RPM_ARCH}.tar.gz"
# --owner/--group so the archive, and therefore the package, does not record the
# build user; %attr in the spec sets the few paths that need anything else.
tar -czf "$PAYLOAD" --owner=root --group=root -C "$ROOT" .

# _with_plugin is what makes the %files entry for /usr/share/whois-server
# conditional — without it, --no-plugin builds would fail on a missing path.
PLUGIN_DEFINE=()
[ "$WITH_PLUGIN" = 1 ] && PLUGIN_DEFINE=(--define "_with_plugin 1")

rpmbuild -bb "$RPM_DIR/$PKG_NAME.spec" \
    --target "$RPM_ARCH" \
    --define "_topdir $TOPDIR" \
    --define "_pkg_version $VERSION" \
    --define "_pkg_release $RPM_RELEASE" \
    "${PLUGIN_DEFINE[@]}"

# ---------------------------------------------------------------------------
# 4. Collect
# ---------------------------------------------------------------------------
mkdir -p "$OUTPUT_DIR"
BUILT="$(find "$TOPDIR/RPMS" -name '*.rpm' -type f | head -n1)"
[ -n "$BUILT" ] || die "rpmbuild reported success but produced no .rpm"

RPM_PATH="$OUTPUT_DIR/$(basename "$BUILT")"
cp "$BUILT" "$RPM_PATH"
( cd "$OUTPUT_DIR" && sha256sum "$(basename "$RPM_PATH")" > "$(basename "$RPM_PATH").sha256" )

echo "==> $RPM_PATH"
rpm -qip "$RPM_PATH" 2>/dev/null | sed 's/^/    /' || :

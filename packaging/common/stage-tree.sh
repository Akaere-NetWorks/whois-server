# shellcheck shell=bash
#
# stage-tree.sh — shared filesystem layout for the .deb and .rpm packages.
#
# Sourced, not executed. Both format-specific build scripts call stage_tree()
# and then do nothing but wrap the result in their own metadata, so the two
# packages cannot drift apart in what they actually install. The systemd unit is
# likewise shared verbatim (packaging/common/whois-server.service) — it reads
# both /etc/default and /etc/sysconfig so one file serves both families.
#
# Layout:
#   /usr/lib/whois-server/                  application + bundled runtime + wwwroot
#   /usr/bin/whois-server                   symlink for interactive use
#   /usr/lib/systemd/system/                the unit
#   $ENV_CONF                               conffile: flags + API tokens (caller picks the path)
#   /var/lib/whois-server/{cache,plugins}   state, chowned by the maintainer scripts
#   /usr/share/whois-server/plugins/        example plugin, for the operator to copy
#   /usr/share/doc/whois-server/            LICENCE and README
#
# /usr/lib rather than /usr/libexec or %{_libdir}: the path is baked into the
# shared unit file, so it has to be one both families agree on. rpmlint will
# note ELF objects under /usr/lib on a 64-bit target; that is cosmetic, and the
# directory is private to this application either way.

PKG_NAME=whois-server

# stage_tree <publish_dir> <root> <env_conf_relpath> <with_plugin 0|1>
#
# Requires REPO_ROOT to be set by the caller. Populates <root> as a complete
# installable tree with no packaging-format metadata in it.
stage_tree() {
    local publish_dir="$1" root="$2" env_conf="$3" with_plugin="$4"

    install -d -m 0755 \
        "$root/usr/lib/$PKG_NAME" \
        "$root/usr/bin" \
        "$root/usr/lib/systemd/system" \
        "$root/$(dirname "$env_conf")" \
        "$root/var/lib/$PKG_NAME/cache" \
        "$root/var/lib/$PKG_NAME/plugins" \
        "$root/usr/share/doc/$PKG_NAME"

    cp -a "$publish_dir/." "$root/usr/lib/$PKG_NAME/"

    # The publish tree carries debug symbols, a createdump helper that is
    # useless without a debugger installed, and libe_sqlite3.a — a static
    # archive that only a linker would want. None of it belongs in a runtime
    # package, and rpmbuild in particular will try to process the .a file.
    find "$root/usr/lib/$PKG_NAME" -name '*.pdb' -delete
    rm -f "$root/usr/lib/$PKG_NAME/createdump" \
          "$root/usr/lib/$PKG_NAME/libe_sqlite3.a"

    chmod 0755 "$root/usr/lib/$PKG_NAME/WhoisServer.Host"
    # Relative link so the tree stays valid under a --instdir / %{buildroot}.
    ln -sf "../lib/$PKG_NAME/WhoisServer.Host" "$root/usr/bin/$PKG_NAME"

    install -m 0644 "$REPO_ROOT/packaging/common/whois-server.service" \
                    "$root/usr/lib/systemd/system/$PKG_NAME.service"
    # 0644 at unpack time, tightened to root:whois-server 0640 by the postinstall
    # scriptlet once the group exists — this file is where API tokens go.
    install -m 0644 "$REPO_ROOT/packaging/common/whois-server.env" "$root/$env_conf"

    install -m 0644 "$REPO_ROOT/LICENSE"   "$root/usr/share/doc/$PKG_NAME/LICENSE"
    install -m 0644 "$REPO_ROOT/COPYRIGHT" "$root/usr/share/doc/$PKG_NAME/COPYRIGHT"
    install -m 0644 "$REPO_ROOT/README.md" "$root/usr/share/doc/$PKG_NAME/README.md"

    # Shipped under /usr/share, not dropped into /var/lib/whois-server/plugins:
    # loading it should be an operator decision, and package-managed files have
    # no business inside a directory the admin also writes to.
    if [ "$with_plugin" = 1 ]; then
        echo "==> building example plugin"
        dotnet build "$REPO_ROOT/plugins-src/Weather/Weather.csproj" -c Release
        if [ -f "$REPO_ROOT/plugins/weather/Weather.dll" ]; then
            install -d -m 0755 "$root/usr/share/$PKG_NAME/plugins/weather"
            install -m 0644 "$REPO_ROOT/plugins/weather/Weather.dll" \
                            "$REPO_ROOT/plugins/weather/plugin.json" \
                            "$root/usr/share/$PKG_NAME/plugins/weather/"
        else
            echo "stage_tree: warning: example plugin not found after build, skipping" >&2
        fi
    fi
}

# publish_host <rid> <version> <features> <out_dir>
#
# Self-contained, matching the release tarballs: a package that depended on a
# system dotnet would need a third-party repository configured first, which is a
# worse install story than ~90 MB of bundled runtime. Not trimmed — the JSON and
# DI paths are reflection-heavy enough that PublishTrimmed is a correctness risk.
publish_host() {
    local rid="$1" version="$2" features="$3" out_dir="$4"
    # shellcheck disable=SC2086  # $features must word-split into separate flags
    dotnet publish "$REPO_ROOT/src/WhoisServer.Host/WhoisServer.Host.csproj" \
        -c Release \
        -r "$rid" \
        --self-contained true \
        -p:VersionPrefix="$version" \
        $features \
        -o "$out_dir"
}

# resolve_version <explicit_version>
#
# Echoes the version to stamp. The explicit flag wins; otherwise the single
# <VersionPrefix> that Directory.Build.props declares (CI overrides that with
# -p:VersionPrefix from the tag, so the same value ends up in the assembly and
# in the package). Both dpkg and rpm require a leading digit.
resolve_version() {
    local version="${1:-}"
    if [ -z "$version" ]; then
        version="$(sed -n 's:.*<VersionPrefix>\(.*\)</VersionPrefix>.*:\1:p' \
                   "$REPO_ROOT/Directory.Build.props" | head -n1)"
        [ -n "$version" ] || {
            echo "could not read <VersionPrefix> from Directory.Build.props; pass --version" >&2
            return 1
        }
    fi
    version="${version#v}"
    case "$version" in
        [0-9]*) ;;
        *) echo "invalid package version '$version': must start with a digit" >&2; return 1 ;;
    esac
    # A '-' separates version from release in RPM and from the Debian revision in
    # dpkg, so a pre-release tag like 1.0.0-rc1 has to become 1.0.0~rc1 to sort
    # correctly (and, for RPM, to be legal at all).
    printf '%s\n' "${version//-/\~}"
}

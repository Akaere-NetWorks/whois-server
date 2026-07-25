#
# whois-server.spec — binary repack of a pre-built `dotnet publish` tree.
#
# Not a from-source spec. %%build is empty on purpose: build-rpm.sh publishes
# the host, stages the exact installed filesystem via
# packaging/common/stage-tree.sh (the same function the .deb uses, so the two
# packages cannot drift), and hands the result over as Source0. %%install
# therefore does nothing but unpack it.
#
# Consequences of repacking a self-contained .NET tree, all deliberate:
#
#   * No debuginfo subpackage. The payload is prebuilt and its symbols were
#     already dropped during staging; leaving debug_package on makes rpmbuild
#     fail looking for sources that were never here.
#   * No stripping or rpath checking. The bundled runtime's .so files ship as
#     Microsoft built them; strip has nothing to gain and a real chance of
#     breaking the CoreCLR debugging shims.
#   * AutoReqProv off. Requires are declared by hand below — partly because the
#     package is routinely cross-built on a Debian-family CI runner whose
#     find-requires does not know Fedora's soname-to-package mapping, and partly
#     because libicu is dlopen'd at runtime and so is invisible to any automatic
#     scanner despite being mandatory.
#
%global debug_package %{nil}
%global __strip /bin/true
%global __brp_strip %{nil}
%global __brp_strip_static_archive %{nil}
%global __brp_strip_comment_note %{nil}
%global __brp_check_rpaths %{nil}
%global __brp_mangle_shebangs %{nil}

# Not %%{_sharedstatedir}: that macro is /var/lib only because Fedora patches it,
# and this spec is routinely built with Debian's rpm, where it keeps the ancient
# GNU default of /usr/com — every %%files entry under it then fails to resolve.
# The path is hardcoded in the shared systemd unit anyway, so a literal is both
# safer and more honest here. Spelled out rather than /var/lib/%%{name} because
# %%global expands its body at definition time, and Name: is not set yet.
%global statedir /var/lib/whois-server

Name:           whois-server
Version:        %{?_pkg_version}%{!?_pkg_version:0.0.1}
Release:        %{?_pkg_release}%{!?_pkg_release:1}%{?dist}
Summary:        Multi-protocol WHOIS server with DN42 support and a web dashboard

License:        AGPL-3.0-or-later
URL:            https://github.com/Akaere-NetWorks/whois-server
Source0:        %{name}-payload-%{version}-%{_target_cpu}.tar.gz

# The payload is already compiled for one architecture; noarch is wrong, and
# ExclusiveArch stops a mismatched build from producing a package that installs
# and then refuses to exec.
ExclusiveArch:  x86_64 aarch64

AutoReqProv:    no
Requires:       glibc
Requires:       libgcc
Requires:       libstdc++
Requires:       zlib
Requires:       openssl-libs
# Mandatory despite being dlopen'd: without ICU the runtime aborts during
# startup unless DOTNET_SYSTEM_GLOBALIZATION_INVARIANT is set.
Requires:       libicu
Requires:       ca-certificates
# Hard dependency, not a recommendation: the DN42 backend shells out to the git
# binary for `clone --depth 1` / `pull`. Build with -p:Feature_Dn42=false to
# drop the need for it. git-core rather than git — /usr/bin/git lives there and
# the full git package drags in perl.
Requires:       git-core
Requires:       systemd
# groupadd/useradd in %%pre.
Requires(pre):  shadow-utils

%description
A WHOIS daemon speaking RFC 3912 on TCP port 43, with a 60-suffix query
dispatcher that fans out to around 46 upstream service integrations (BGP,
geolocation, DNS, package registries and more), first-class DN42 registry
support backed by a local git mirror, and a web dashboard with a JSON API.

Runs as a single .NET 10 process under systemd as an unprivileged service
account, using CAP_NET_BIND_SERVICE to bind the privileged WHOIS port. The
.NET runtime is bundled, so no system dotnet installation is required.

%prep
# -c -T: create the build directory and skip the default unpack, then unpack
# Source0 by hand so its paths stay rooted where %%install expects them.
%setup -q -c -T
tar -xzf %{SOURCE0}

%build
# Nothing to build — see the header.

%install
rm -rf %{buildroot}
mkdir -p %{buildroot}
cp -a ./* %{buildroot}/

%files
%dir %{_datadir}/doc/%{name}
%license %{_datadir}/doc/%{name}/LICENSE
%doc %{_datadir}/doc/%{name}/COPYRIGHT
%doc %{_datadir}/doc/%{name}/README.md
%{_prefix}/lib/%{name}/
%{_bindir}/%{name}
%{_prefix}/lib/systemd/system/%{name}.service
%if %{defined _with_plugin}
%{_datadir}/%{name}/
%endif
# noreplace so an upgrade never clobbers the operator's API tokens. Group-owned
# by the service account and not world-readable, because that is where they go.
%config(noreplace) %attr(0640,root,whois-server) %{_sysconfdir}/sysconfig/%{name}
# Ownership is declared by name rather than fixed up in %%post: %%pre creates the
# account before any file is unpacked, so rpm can resolve these at install time —
# and unlike a %%post chown, it survives an upgrade without extra scriptlets.
%dir %attr(0750,whois-server,whois-server) %{statedir}
%dir %attr(0750,whois-server,whois-server) %{statedir}/cache
%dir %attr(0750,whois-server,whois-server) %{statedir}/plugins

%pre
# System account, no login shell, home pointed at the state directory but not
# created by useradd — the package ships it as a %%dir.
getent group whois-server >/dev/null || groupadd --system whois-server
getent passwd whois-server >/dev/null || \
    useradd --system --gid whois-server \
            --home-dir %{statedir} --no-create-home \
            --shell /sbin/nologin \
            --comment "whois-server daemon" whois-server
exit 0

%post
# Hand-written rather than %%systemd_post: this spec is routinely built on hosts
# without systemd-rpm-macros installed, where an undefined macro would survive
# into the scriptlet as literal text and silently do nothing.
systemctl daemon-reload >/dev/null 2>&1 || :
if [ "$1" -eq 1 ]; then
    # Fresh install: match the .deb, which enables and starts the unit.
    systemctl enable --now whois-server.service >/dev/null 2>&1 || :
else
    # Upgrade: restart only if it was already running, so an operator who
    # deliberately stopped the service does not find it back up.
    systemctl try-restart whois-server.service >/dev/null 2>&1 || :
fi
exit 0

%preun
# $1 is 0 on erase and 1 on upgrade; only a real removal should stop the daemon.
if [ "$1" -eq 0 ]; then
    systemctl --no-reload disable --now whois-server.service >/dev/null 2>&1 || :
fi
exit 0

%postun
systemctl daemon-reload >/dev/null 2>&1 || :
# Deliberately no user deletion and no rm -rf of the state directory. RPM has no
# equivalent of dpkg's purge, so an erase here is closer to `apt remove` than to
# `apt purge`: anything the daemon wrote under /var/lib/whois-server (the DN42
# clone, stats, operator-installed plugins) is left for the admin to remove.
exit 0

%changelog
# Intentionally empty: entries are generated from git tags at release time
# rather than maintained by hand in-tree.

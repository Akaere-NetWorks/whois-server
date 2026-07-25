// WHOIS Server dashboard — built with the real @cloudflare/kumo component library (see
// /vendor/kumo), loaded as a plain <script> with no build step: this file uses
// React.createElement directly (no JSX) since there's no bundler/transpiler in this project.
//
// Deliberately does NOT use Kumo's Sidebar/TimeseriesChart/Chart compound components:
// - TimeseriesChart/Chart require an `echarts` instance to be passed in by the caller, and
//   Apache ECharts isn't part of this design-system export — pulling it in from a CDN would
//   reintroduce the external dependency this project has otherwise avoided everywhere else, so
//   trend data is rendered with a small hand-rolled SVG sparkline instead.
// - Sidebar is a large compound component (Provider/MenuButton/Collapsible/render-prop merging)
//   whose exact runtime contract could only be fully verified by driving it in a browser; without
//   that in this environment, a plain flex column of real Button components styled with Kumo's
//   own semantic tokens (bg-kumo-base, border-kumo-line, ...) gets the same visual result with
//   much less risk of a silent runtime mismatch.
(function () {
  "use strict";

  var h = React.createElement;
  var useState = React.useState;
  var useEffect = React.useEffect;
  var useCallback = React.useCallback;

  var Kumo = window.Kumo;
  var Button = Kumo.Button;
  var Text = Kumo.Text;
  var Badge = Kumo.Badge;
  var Input = Kumo.Input;
  var LayerCard = Kumo.LayerCard;
  var Table = Kumo.Table;
  var Code = Kumo.Code;
  var Banner = Kumo.Banner;
  var Loader = Kumo.Loader;

  function formatBytes(bytes) {
    if (!bytes || bytes <= 0) return "0 B";
    var units = ["B", "KB", "MB", "GB", "TB"];
    var i = Math.min(units.length - 1, Math.floor(Math.log(bytes) / Math.log(1024)));
    var value = bytes / Math.pow(1024, i);
    return (i === 0 ? value.toFixed(0) : value.toFixed(1)) + " " + units[i];
  }

  function formatNumber(n) {
    return (n || 0).toLocaleString();
  }

  // Minimal hand-rolled sparkline: no chart library dependency, just an SVG polyline scaled to
  // its own min/max.
  function Sparkline(props) {
    var points = props.points || [];
    var width = 560;
    var height = 72;
    var max = Math.max.apply(null, points.concat([1]));
    var step = points.length > 1 ? width / (points.length - 1) : width;

    var coords = points.map(function (v, i) {
      var x = i * step;
      var y = height - (max > 0 ? (v / max) * (height - 8) : 0) - 4;
      return x.toFixed(1) + "," + y.toFixed(1);
    });

    var areaPath =
      "M0," + height + " L" + coords.join(" L") + " L" + width + "," + height + " Z";

    return h(
      "svg",
      { viewBox: "0 0 " + width + " " + height, width: "100%", height: height, preserveAspectRatio: "none" },
      h("path", { d: areaPath, fill: "var(--color-kumo-brand)", opacity: 0.12 }),
      h("polyline", {
        points: coords.join(" "),
        fill: "none",
        stroke: "var(--color-kumo-brand)",
        strokeWidth: 2,
        strokeLinejoin: "round",
        strokeLinecap: "round",
      })
    );
  }

  function StatTile(props) {
    return h(
      LayerCard,
      { className: "p-4 flex flex-col gap-1" },
      h(Text, { variant: "secondary" }, props.label),
      h(Text, { variant: "heading2" }, props.value)
    );
  }

  function QueryResult(props) {
    var result = props.result;

    if (!result.success) {
      return h(Banner, {
        variant: "error",
        title: "Query failed",
        description: result.error || "Unknown error",
      });
    }

    var fields = result.fields || [];

    return h(
      "div",
      { className: "flex flex-col gap-3" },
      h(
        "div",
        { className: "flex items-center gap-2" },
        h(Badge, { variant: "secondary" }, result.query_type),
        h(
          Text,
          { variant: "mono-secondary" },
          result.metadata.processing_time_ms.toFixed(1) + " ms"
        )
      ),
      h(Code, { code: result.raw_output || "(empty response)", lang: "bash" }),
      fields.length > 0
        ? h(
            LayerCard,
            { className: "p-0" },
            h(
              Table,
              null,
              h(
                Table.Header,
                null,
                h(Table.Row, null, h(Table.Head, null, "Field"), h(Table.Head, null, "Value"))
              ),
              h(
                Table.Body,
                null,
                fields.map(function (f, i) {
                  return h(
                    Table.Row,
                    { key: i },
                    h(Table.Cell, null, f.name),
                    h(Table.Cell, { className: "break-all" }, f.value)
                  );
                })
              )
            )
          )
        : null
    );
  }

  function Dashboard() {
    var statsState = useState(null);
    var stats = statsState[0];
    var setStats = statsState[1];

    var errorState = useState(null);
    var statsError = errorState[0];
    var setStatsError = errorState[1];

    var queryState = useState("");
    var query = queryState[0];
    var setQuery = queryState[1];

    var loadingState = useState(false);
    var loading = loadingState[0];
    var setLoading = loadingState[1];

    var resultState = useState(null);
    var result = resultState[0];
    var setResult = resultState[1];

    useEffect(function () {
      fetch("/api/stats")
        .then(function (r) {
          return r.json();
        })
        .then(setStats)
        .catch(function (e) {
          setStatsError(String(e));
        });
    }, []);

    var runQuery = useCallback(
      function () {
        var trimmed = query.trim();
        if (!trimmed) return;
        setLoading(true);
        setResult(null);
        fetch("/api/whois?q=" + encodeURIComponent(trimmed))
          .then(function (r) {
            return r.json();
          })
          .then(setResult)
          .catch(function (e) {
            setResult({ success: false, error: String(e) });
          })
          .finally(function () {
            setLoading(false);
          });
      },
      [query]
    );

    return h(
      "div",
      { className: "flex flex-col gap-6 p-6 max-w-5xl mx-auto w-full" },
      h(Text, { variant: "heading1", as: "h1" }, "WHOIS Server"),
      h(
        Text,
        { variant: "secondary" },
        "Live query statistics and an interactive query tester."
      ),
      statsError
        ? h(Banner, {
            variant: "error",
            title: "Failed to load statistics",
            description: statsError,
          })
        : null,
      stats
        ? h(
            "div",
            { className: "grid grid-cols-1 md:grid-cols-2 gap-4" },
            h(StatTile, { label: "Total requests", value: formatNumber(stats.totalRequests) }),
            h(StatTile, {
              label: "Total data served",
              value: formatBytes(stats.totalBytesServed),
            })
          )
        : h(Loader, null),
      stats
        ? h(
            "div",
            { className: "grid grid-cols-1 md:grid-cols-2 gap-4" },
            h(
              LayerCard,
              { className: "p-4" },
              h(Text, { variant: "heading3" }, "Requests — last 24h"),
              h(Sparkline, {
                points: stats.hourly24h.map(function (p) {
                  return p.requests;
                }),
              })
            ),
            h(
              LayerCard,
              { className: "p-4" },
              h(Text, { variant: "heading3" }, "Requests — last 30d"),
              h(Sparkline, {
                points: stats.daily30d.map(function (p) {
                  return p.requests;
                }),
              })
            )
          )
        : null,
      h(
        LayerCard,
        { className: "p-4 flex flex-col gap-3" },
        h(Text, { variant: "heading3" }, "Try a query"),
        h(Input, {
          "aria-label": "Query",
          value: query,
          onChange: function (e) {
            setQuery(e.target.value);
          },
          onKeyDown: function (e) {
            if (e.key === "Enter") runQuery();
          },
          placeholder: "example.com, AS213605, 8.8.8.8-GEO, HELP, ...",
        }),
        h(
          "div",
          { className: "flex gap-2" },
          h(
            Button,
            { variant: "primary", onClick: runQuery, loading: loading },
            "Run query"
          )
        ),
        result ? h(QueryResult, { result: result }) : null
      )
    );
  }

  function Docs() {
    var endpoints = [
      { method: "GET", path: "/api/whois?q=<query>", desc: "JSON-formatted WHOIS response" },
      { method: "POST", path: "/api/whois", desc: 'Same as above; JSON body {"q": "..."}' },
      { method: "GET", path: "/raw/<query>", desc: "Raw WHOIS text output (text/plain)" },
      { method: "GET", path: "/api/stats", desc: "Server statistics (24h/30d series)" },
      { method: "GET", path: "/api/openapi.json", desc: "OpenAPI 3 specification" },
    ];

    return h(
      "div",
      { className: "flex flex-col gap-6 p-6 max-w-4xl mx-auto w-full" },
      h(Text, { variant: "heading1", as: "h1" }, "API Reference"),
      h(
        LayerCard,
        { className: "p-0" },
        h(
          Table,
          null,
          h(
            Table.Header,
            null,
            h(
              Table.Row,
              null,
              h(Table.Head, null, "Method"),
              h(Table.Head, null, "Path"),
              h(Table.Head, null, "Description")
            )
          ),
          h(
            Table.Body,
            null,
            endpoints.map(function (e, i) {
              return h(
                Table.Row,
                { key: i },
                h(
                  Table.Cell,
                  null,
                  h(Badge, { variant: e.method === "GET" ? "success" : "info" }, e.method)
                ),
                h(Table.Cell, null, h("code", null, e.path)),
                h(Table.Cell, null, e.desc)
              );
            })
          )
        )
      ),
      h(
        LayerCard,
        { className: "p-4 flex flex-col gap-3" },
        h(Text, { variant: "heading3" }, "Examples"),
        h(Code, {
          lang: "bash",
          code:
            'curl "http://localhost:9999/api/whois?q=example.com"\n' +
            'curl "http://localhost:9999/raw/AS213605"\n' +
            'curl "http://localhost:9999/api/stats"',
        })
      )
    );
  }

  function NavButton(props) {
    return h(
      Button,
      {
        variant: props.active ? "primary" : "ghost",
        className: "justify-start w-full",
        onClick: function () {
          window.location.assign(props.href);
        },
      },
      props.label
    );
  }

  function App() {
    var isDocs = window.location.pathname.indexOf("/docs") === 0;

    return h(
      "div",
      { className: "flex min-h-screen w-full" },
      h(
        "nav",
        {
          className:
            "w-56 shrink-0 border-r border-kumo-line bg-kumo-base p-4 flex flex-col gap-2",
        },
        h(Text, { variant: "heading3", className: "mb-2" }, "WHOIS Server"),
        h(NavButton, { label: "Dashboard", href: "/", active: !isDocs }),
        h(NavButton, { label: "API Reference", href: "/docs", active: isDocs })
      ),
      h("main", { className: "flex-1 overflow-auto" }, isDocs ? h(Docs) : h(Dashboard))
    );
  }

  ReactDOM.createRoot(document.getElementById("root")).render(h(App));
})();

/*
 * WHOIS Server with DN42 Support
 * Copyright (C) 2025 Akaere Networks
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

use axum::{
    extract::State,
    response::{Html, IntoResponse, Json},
    routing::get,
    Router,
};
use tower_http::cors::CorsLayer;
use crate::stats::{StatsState, get_stats_response};

pub async fn run_web_server(stats: StatsState, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new()
        .route("/", get(dashboard))
        .route("/api/stats", get(get_stats_api))
        .layer(CorsLayer::permissive())
        .with_state(stats);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn dashboard() -> impl IntoResponse {
    let html = r#"
<!DOCTYPE html>
<html lang="en" data-theme="dark" id="html-root">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Akaere Networks Whois Server</title>
    <link href="https://cdn.jsdelivr.net/npm/daisyui@5" rel="stylesheet" type="text/css" />
    <script src="https://cdn.jsdelivr.net/npm/@tailwindcss/browser@4"></script>
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
    <style>
        :root {
            --cute-pink-50: #fef7f7;
            --cute-pink-100: #feeaea;
            --cute-pink-200: #fdd5d5;
            --cute-pink-300: #fcb5b5;
            --cute-pink-400: #f98d8d;
            --cute-pink-500: #f472b6;
            --cute-pink-600: #e879a7;
            --cute-pink-700: #d97398;
            --cute-pink-800: #c96d89;
            --cute-pink-900: #b8677a;
        }
        
        .title-gradient {
            background: linear-gradient(135deg, var(--cute-pink-400), var(--cute-pink-500));
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            background-clip: text;
            filter: brightness(1.1);
        }
        
        [data-theme="light"] .title-gradient {
            background: linear-gradient(135deg, var(--cute-pink-500), var(--cute-pink-600));
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            background-clip: text;
        }
        
        .pink-command {
            background: linear-gradient(135deg, var(--cute-pink-50), var(--cute-pink-100));
            border: 2px solid var(--cute-pink-200);
            color: var(--cute-pink-700);
            border-radius: 12px;
        }
        
        [data-theme="light"] .pink-command {
            background: linear-gradient(135deg, var(--cute-pink-50), var(--cute-pink-100));
            border: 2px solid var(--cute-pink-200);
            color: var(--cute-pink-600);
        }
        
        .btn-pink {
            background: linear-gradient(135deg, var(--cute-pink-300), var(--cute-pink-400));
            border: none;
            color: white;
            transition: all 0.3s ease;
            border-radius: 12px;
            font-weight: 600;
        }
        
        .btn-pink:hover {
            background: linear-gradient(135deg, var(--cute-pink-400), var(--cute-pink-500));
            transform: translateY(-2px);
            box-shadow: 0 6px 20px rgba(244, 114, 182, 0.25);
            color: white;
        }
        
        .btn-pink-outline {
            background: rgba(244, 114, 182, 0.05);
            border: 2px solid var(--cute-pink-300);
            color: var(--cute-pink-600);
            border-radius: 12px;
            font-weight: 600;
            transition: all 0.3s ease;
        }
        
        .btn-pink-outline:hover {
            background: linear-gradient(135deg, var(--cute-pink-200), var(--cute-pink-300));
            border-color: var(--cute-pink-400);
            color: var(--cute-pink-700);
            transform: translateY(-1px);
        }
        
        [data-theme="light"] .btn-pink-outline {
            border-color: var(--cute-pink-300);
            color: var(--cute-pink-600);
            background: rgba(244, 114, 182, 0.03);
        }
        
        [data-theme="light"] .btn-pink-outline:hover {
            background: linear-gradient(135deg, var(--cute-pink-100), var(--cute-pink-200));
            border-color: var(--cute-pink-400);
            color: var(--cute-pink-700);
        }
    </style>
</head>
<body class="bg-base-200 min-h-screen">
    <div class="container mx-auto px-4 py-8 max-w-6xl">
        <!-- Header -->
        <div class="text-center mb-12">
            <div class="flex justify-between items-center mb-6">
                <div></div> <!-- Spacer for centering -->
                <h1 class="text-5xl font-bold title-gradient">Akaere Networks Whois Server</h1>
                <!-- Theme Toggle -->
                <div class="dropdown dropdown-end">
                    <div tabindex="0" role="button" class="btn btn-ghost btn-circle" id="theme-toggle">
                        <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24" id="theme-icon">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z"/>
                        </svg>
                    </div>
                    <ul tabindex="0" class="dropdown-content z-[1] menu p-2 shadow bg-base-100 rounded-box w-32">
                        <li><a onclick="setTheme('light')">
                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z"/>
                            </svg>
                            Light
                        </a></li>
                        <li><a onclick="setTheme('dark')">
                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z"/>
                            </svg>
                            Dark
                        </a></li>
                        <li><a onclick="setTheme('auto')">
                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"/>
                            </svg>
                            Auto
                        </a></li>
                    </ul>
                </div>
            </div>
            <p class="text-xl text-base-content/70 max-w-2xl mx-auto">
                High-performance WHOIS server with comprehensive DN42 support, geo-location services, and advanced query capabilities
            </p>
        </div>

        <!-- Usage Instructions -->
        <div class="card bg-base-100 shadow-xl mb-8">
            <div class="card-body">
                <h2 class="card-title text-2xl mb-4">
                    <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
                    </svg>
                    Quick Start
                </h2>
                
                <div class="pink-command p-4 rounded-lg mb-4">
                    <code class="font-mono font-semibold">whois -h whois.akae.re [query]</code>
                </div>

                <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div class="space-y-3">
                        <h3 class="font-semibold text-lg">Standard Queries</h3>
                        <div class="space-y-2 text-sm">
                            <div><code class="bg-base-200 px-2 py-1 rounded">google.com</code> - Domain lookup</div>
                            <div><code class="bg-base-200 px-2 py-1 rounded">8.8.8.8</code> - IPv4 address</div>
                            <div><code class="bg-base-200 px-2 py-1 rounded">AS213605</code> - ASN lookup</div>
                        </div>
                    </div>
                    
                    <div class="space-y-3">
                        <h3 class="font-semibold text-lg">Special Parameters</h3>
                        <div class="space-y-2 text-sm">
                            <div><code class="bg-base-200 px-2 py-1 rounded">AS213605-EMAIL</code> - Email search</div>
                            <div><code class="bg-base-200 px-2 py-1 rounded">8.8.8.8-GEO</code> - Geo location</div>
                            <div><code class="bg-base-200 px-2 py-1 rounded">8.8.8.8-RIRGEO</code> - RIR geo data</div>
                            <div><code class="bg-base-200 px-2 py-1 rounded">AS213605-PREFIXES</code> - ASN prefixes</div>
                            <div><code class="bg-base-200 px-2 py-1 rounded">AS213605-BGPTOOL</code> - BGP tools</div>
                            <div><code class="bg-base-200 px-2 py-1 rounded">1.1.1.0-RADB</code> - RADB query</div>
                            <div><code class="bg-base-200 px-2 py-1 rounded">192.0.2.0/24-IRR</code> - IRR Explorer</div>
                            <div><code class="bg-base-200 px-2 py-1 rounded">1.1.1.0-LG</code> - Looking Glass</div>
                        </div>
                    </div>
                </div>

                <div class="mt-6">
                    <h3 class="font-semibold text-lg mb-3">Advanced Query Features</h3>
                    <div class="grid grid-cols-1 md:grid-cols-3 gap-4 text-sm">
                        <div class="space-y-2">
                            <h4 class="font-medium text-cute-pink-600">Network Analysis</h4>
                            <div><code class="bg-base-200 px-2 py-1 rounded text-xs">-RADB</code> - Routing Assets Database</div>
                            <div><code class="bg-base-200 px-2 py-1 rounded text-xs">-IRR</code> - IRR Explorer with RPKI</div>
                            <div><code class="bg-base-200 px-2 py-1 rounded text-xs">-LG</code> - Looking Glass (BIRD format)</div>
                        </div>
                        <div class="space-y-2">
                            <h4 class="font-medium text-cute-pink-600">Geographic Data</h4>
                            <div><code class="bg-base-200 px-2 py-1 rounded text-xs">-GEO</code> - IP geolocation</div>
                            <div><code class="bg-base-200 px-2 py-1 rounded text-xs">-RIRGEO</code> - RIR geographic data</div>
                        </div>
                        <div class="space-y-2">
                            <h4 class="font-medium text-cute-pink-600">Contact & BGP</h4>
                            <div><code class="bg-base-200 px-2 py-1 rounded text-xs">-EMAIL</code> - Contact search</div>
                            <div><code class="bg-base-200 px-2 py-1 rounded text-xs">-BGPTOOL</code> - BGP information</div>
                            <div><code class="bg-base-200 px-2 py-1 rounded text-xs">-PREFIXES</code> - ASN prefixes</div>
                        </div>
                    </div>
                </div>

                <div class="alert alert-info mt-4">
                    <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                    </svg>
                    <span>DN42 networks are automatically detected and routed to appropriate servers</span>
                </div>
            </div>
        </div>



        <!-- Statistics Cards -->
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
            <div class="stat bg-base-100 shadow-xl rounded-lg">
                <div class="stat-figure text-primary">
                    <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"/>
                    </svg>
                </div>
                <div class="stat-title">Total Requests</div>
                <div class="stat-value text-primary" id="total-requests">-</div>
            </div>

            <div class="stat bg-base-100 shadow-xl rounded-lg">
                <div class="stat-figure text-secondary">
                    <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4"/>
                    </svg>
                </div>
                <div class="stat-title">Total Data Served</div>
                <div class="stat-value text-secondary" id="total-data">-</div>
                <div class="stat-desc">KB</div>
            </div>

            <div class="stat bg-base-100 shadow-xl rounded-lg">
                <div class="stat-figure text-accent">
                    <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"/>
                    </svg>
                </div>
                <div class="stat-title">Today's Requests</div>
                <div class="stat-value text-accent" id="today-requests">-</div>
            </div>

            <div class="stat bg-base-100 shadow-xl rounded-lg">
                <div class="stat-figure text-warning">
                    <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/>
                    </svg>
                </div>
                <div class="stat-title">Today's Data</div>
                <div class="stat-value text-warning" id="today-data">-</div>
                <div class="stat-desc">KB</div>
            </div>
        </div>

        <!-- Chart Period Selection -->
        <div class="text-center mb-6">
            <div class="btn-group">
                <button class="btn btn-pink" id="btn24h" onclick="switchPeriod('24h')">Last 24 Hours</button>
                <button class="btn btn-pink-outline" id="btn30d" onclick="switchPeriod('30d')">Last 30 Days</button>
            </div>
        </div>

        <!-- Charts -->
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-8">
            <div class="card bg-base-100 shadow-xl">
                <div class="card-body">
                    <h2 class="card-title" id="requestsChartTitle">Daily Requests - Last 24 Hours</h2>
                    <div class="h-64">
                        <canvas id="requestsChart"></canvas>
                    </div>
                </div>
            </div>

            <div class="card bg-base-100 shadow-xl">
                <div class="card-body">
                    <h2 class="card-title" id="dataChartTitle">Daily Data Transfer (KB) - Last 24 Hours</h2>
                    <div class="h-64">
                        <canvas id="dataChart"></canvas>
                    </div>
                </div>
            </div>
        </div>

        <!-- Footer -->
        <footer class="text-center mt-12 text-base-content/60">
            <p>&copy; 2025 Akaere Networks. Licensed under AGPL-3.0-or-later.</p>
        </footer>
    </div>

    <script>
        let requestsChart, dataChart;
        let currentPeriod = '24h';
        let currentData = null;

        async function fetchStats() {
            try {
                const response = await fetch('/api/stats');
                const data = await response.json();
                currentData = data;
                
                // Update stat cards
                document.getElementById('total-requests').textContent = data.total_requests.toLocaleString();
                document.getElementById('total-data').textContent = data.total_kb_served.toFixed(2);
                
                // Calculate today's stats from daily data (more reliable)
                const today = new Date().toISOString().split('T')[0];
                const todayStats = data.daily_stats_30d.find(s => s.date === today);
                
                const todayRequests = todayStats ? todayStats.requests : 0;
                const todayData = todayStats ? todayStats.kb_served : 0;
                
                document.getElementById('today-requests').textContent = todayRequests.toLocaleString();
                document.getElementById('today-data').textContent = todayData.toFixed(2);
                
                // Update charts based on current period
                updateCharts();
            } catch (error) {
                console.error('Failed to fetch stats:', error);
            }
        }

        function switchPeriod(period) {
            currentPeriod = period;
            
            // Update button styles
            document.getElementById('btn24h').className = period === '24h' ? 'btn btn-pink' : 'btn btn-pink-outline';
            document.getElementById('btn30d').className = period === '30d' ? 'btn btn-pink' : 'btn btn-pink-outline';
            
            // Update chart titles
            const periodText = period === '24h' ? 'Last 24 Hours' : 'Last 30 Days';
            document.getElementById('requestsChartTitle').textContent = `Daily Requests - ${periodText}`;
            document.getElementById('dataChartTitle').textContent = `Daily Data Transfer (KB) - ${periodText}`;
            
            // Update charts
            updateCharts();
        }

        function getThemeColors() {
            const isDark = document.documentElement.getAttribute('data-theme') === 'dark';
            return {
                textColor: isDark ? '#a6adba' : '#374151',
                gridColor: isDark ? '#374151' : '#e5e7eb',
                primaryColor: isDark ? 'rgb(249, 141, 141)' : 'rgb(217, 115, 152)', // Cute light pink
                primaryBg: isDark ? 'rgba(249, 141, 141, 0.12)' : 'rgba(217, 115, 152, 0.08)',
                successColor: isDark ? 'rgb(252, 181, 181)' : 'rgb(232, 121, 167)', // Very light pink
                successBg: isDark ? 'rgba(252, 181, 181, 0.7)' : 'rgba(232, 121, 167, 0.6)'
            };
        }

        function updateCharts() {
            if (!currentData) return;
            
            const colors = getThemeColors();
            const dailyStats = currentPeriod === '24h' ? currentData.daily_stats_24h : currentData.daily_stats_30d;
            const dates = dailyStats.map(s => {
                if (currentPeriod === '24h') {
                    return s.date; // Already formatted as "HH:00"
                } else {
                    return s.date.split('-')[1] + '/' + s.date.split('-')[2]; // MM/DD format for 30 days
                }
            });
            const requests = dailyStats.map(s => s.requests);
            const kbServed = dailyStats.map(s => s.kb_served);

            // Requests Chart
            if (requestsChart) requestsChart.destroy();
            const ctx1 = document.getElementById('requestsChart').getContext('2d');
            requestsChart = new Chart(ctx1, {
                type: 'line',
                data: {
                    labels: dates,
                    datasets: [{
                        label: 'Daily Requests',
                        data: requests,
                        borderColor: colors.primaryColor,
                        backgroundColor: colors.primaryBg,
                        tension: 0.4,
                        fill: true,
                        pointRadius: 3,
                        pointHoverRadius: 5
                    }]
                },
                options: {
                    responsive: true,
                    maintainAspectRatio: false,
                    interaction: {
                        intersect: false,
                        mode: 'index'
                    },
                    plugins: {
                        legend: {
                            labels: { color: colors.textColor }
                        }
                    },
                    scales: {
                        x: { 
                            ticks: { 
                                color: colors.textColor,
                                maxTicksLimit: currentPeriod === '24h' ? 12 : 15
                            },
                            grid: { color: colors.gridColor }
                        },
                        y: { 
                            ticks: { color: colors.textColor },
                            grid: { color: colors.gridColor },
                            beginAtZero: true
                        }
                    }
                }
            });

            // Data Chart
            if (dataChart) dataChart.destroy();
            const ctx2 = document.getElementById('dataChart').getContext('2d');
            dataChart = new Chart(ctx2, {
                type: 'bar',
                data: {
                    labels: dates,
                    datasets: [{
                        label: 'Data Served (KB)',
                        data: kbServed,
                        backgroundColor: colors.successBg,
                        borderColor: colors.successColor,
                        borderWidth: 1
                    }]
                },
                options: {
                    responsive: true,
                    maintainAspectRatio: false,
                    interaction: {
                        intersect: false,
                        mode: 'index'
                    },
                    plugins: {
                        legend: {
                            labels: { color: colors.textColor }
                        }
                    },
                    scales: {
                        x: { 
                            ticks: { 
                                color: colors.textColor,
                                maxTicksLimit: currentPeriod === '24h' ? 12 : 15
                            },
                            grid: { color: colors.gridColor }
                        },
                        y: { 
                            ticks: { color: colors.textColor },
                            grid: { color: colors.gridColor },
                            beginAtZero: true
                        }
                    }
                }
            });
        }

        // Theme management
        function getSystemTheme() {
            return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
        }

        function setTheme(theme) {
            const htmlRoot = document.getElementById('html-root');
            const themeIcon = document.getElementById('theme-icon');
            
            let actualTheme = theme;
            if (theme === 'auto') {
                actualTheme = getSystemTheme();
            }
            
            htmlRoot.setAttribute('data-theme', actualTheme);
            localStorage.setItem('theme', theme);
            
            // Update theme icon
            if (actualTheme === 'dark') {
                themeIcon.innerHTML = '<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z"/>';
            } else {
                themeIcon.innerHTML = '<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z"/>';
            }
            
            // Update charts with new theme colors
            if (currentData) {
                updateCharts();
            }
        }

        function initTheme() {
            const savedTheme = localStorage.getItem('theme') || 'auto';
            setTheme(savedTheme);
            
            // Listen for system theme changes
            window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {
                const currentTheme = localStorage.getItem('theme') || 'auto';
                if (currentTheme === 'auto') {
                    setTheme('auto');
                }
            });
        }

        // Initialize theme before anything else
        initTheme();

        // Initial fetch and setup auto-refresh
        fetchStats();
        setInterval(fetchStats, 30000); // Refresh every 30 seconds
    </script>
</body>
</html>
    "#;

    Html(html)
}

async fn get_stats_api(State(stats): State<StatsState>) -> impl IntoResponse {
    match get_stats_response(&stats).await {
        response => Json(response),
    }
} 
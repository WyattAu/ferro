use leptos::*;
use leptos_router::*;

use crate::components::header::Header;
use crate::components::sidebar::Sidebar;
use crate::pages::audit::AuditPage;
use crate::pages::dashboard::DashboardPage;
use crate::pages::federation::FederationPage;
use crate::pages::login::LoginPage;
use crate::pages::monitoring::MonitoringPage;
use crate::pages::settings::SettingsPage;
use crate::pages::storage::StoragePage;
use crate::pages::users::UsersPage;
use crate::pages::webhooks::WebhooksPage;
use crate::state::provide_api_state;

#[component]
pub fn App() -> impl IntoView {
    let api = provide_api_state();

    view! {
        <style>{GLOBAL_CSS}</style>
        <Router>
            {move || {
                let connected = api.with(|a| a.is_connected());
                view! {
                    <div class="app-layout">
                        <Sidebar api=api/>
                        <div class="main-content">
                            <Header api=api/>
                            <div class="page-content">
                                <Routes>
                                    <Route path="/" view=move || {
                                        if connected {
                                            view! { <DashboardPage api=api/> }
                                        } else {
                                            view! { <LoginPage api=api/> }
                                        }
                                    }/>
                                    <Route path="/login" view=move || view! { <LoginPage api=api/> }/>
                                    <Route path="/users" view=move || view! { <UsersPage api=api/> }/>
                                    <Route path="/storage" view=move || view! { <StoragePage api=api/> }/>
                                    <Route path="/monitoring" view=move || view! { <MonitoringPage api=api/> }/>
                                    <Route path="/settings" view=move || view! { <SettingsPage api=api/> }/>
                                    <Route path="/federation" view=move || view! { <FederationPage api=api/> }/>
                                    <Route path="/webhooks" view=move || view! { <WebhooksPage api=api/> }/>
                                    <Route path="/audit" view=move || view! { <AuditPage api=api/> }/>
                                </Routes>
                            </div>
                        </div>
                    </div>
                }
            }}
        </Router>
    }
}

const GLOBAL_CSS: &str = r#"
:root {
    --bg-primary: #ffffff;
    --bg-secondary: #f8f9fa;
    --bg-sidebar: #1e293b;
    --bg-sidebar-hover: #334155;
    --text-primary: #1e293b;
    --text-secondary: #64748b;
    --text-sidebar: #e2e8f0;
    --text-sidebar-muted: #94a3b8;
    --border-color: #e2e8f0;
    --accent: #3b82f6;
    --accent-hover: #2563eb;
    --success: #22c55e;
    --warning: #f59e0b;
    --danger: #ef4444;
    --info: #06b6d4;
    --neutral: #64748b;
    --radius: 6px;
    --radius-lg: 10px;
    --shadow: 0 1px 3px rgba(0,0,0,0.08);
    --shadow-lg: 0 4px 12px rgba(0,0,0,0.1);
    --sidebar-width: 240px;
    --header-height: 56px;
}

@media (prefers-color-scheme: dark) {
    :root {
        --bg-primary: #0f172a;
        --bg-secondary: #1e293b;
        --bg-sidebar: #020617;
        --bg-sidebar-hover: #0f172a;
        --text-primary: #e2e8f0;
        --text-secondary: #94a3b8;
        --text-sidebar: #e2e8f0;
        --text-sidebar-muted: #64748b;
        --border-color: #334155;
        --shadow: 0 1px 3px rgba(0,0,0,0.3);
        --shadow-lg: 0 4px 12px rgba(0,0,0,0.4);
    }
}

*, *::before, *::after { margin: 0; padding: 0; box-sizing: border-box; }

body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, Roboto, sans-serif;
    background: var(--bg-primary);
    color: var(--text-primary);
    line-height: 1.5;
    -webkit-font-smoothing: antialiased;
}

.app-layout { display: flex; min-height: 100vh; }

/* Sidebar */
.sidebar {
    width: var(--sidebar-width);
    background: var(--bg-sidebar);
    color: var(--text-sidebar);
    position: fixed;
    top: 0; left: 0; bottom: 0;
    display: flex;
    flex-direction: column;
    z-index: 100;
    overflow-y: auto;
}

.sidebar-header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 20px 16px 16px;
    border-bottom: 1px solid rgba(255,255,255,0.08);
}

.sidebar-brand {
    font-size: 16px;
    font-weight: 700;
    letter-spacing: -0.01em;
}

.sidebar-nav { flex: 1; padding: 8px; }

.nav-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 12px;
    border-radius: var(--radius);
    color: var(--text-sidebar-muted);
    text-decoration: none;
    font-size: 14px;
    font-weight: 500;
    transition: background 0.15s, color 0.15s;
    cursor: pointer;
    margin-bottom: 2px;
}

.nav-item:hover {
    background: var(--bg-sidebar-hover);
    color: var(--text-sidebar);
}

.nav-item.nav-active {
    background: var(--accent);
    color: #ffffff;
}

.nav-icon { display: flex; align-items: center; flex-shrink: 0; }

.sidebar-footer {
    padding: 12px 16px;
    border-top: 1px solid rgba(255,255,255,0.08);
}

.sidebar-server {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 0;
    font-size: 12px;
    color: var(--text-sidebar-muted);
}

.sidebar-connected .server-status-dot { background: var(--success); }

.server-status-dot {
    width: 8px; height: 8px;
    border-radius: 50%;
    background: var(--danger);
    flex-shrink: 0;
}

.server-url {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
}

.sidebar-disconnect {
    width: 100%;
    padding: 6px 12px;
    border: 1px solid rgba(255,255,255,0.12);
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-sidebar-muted);
    font-size: 12px;
    cursor: pointer;
    transition: all 0.15s;
}

.sidebar-disconnect:hover:not(:disabled) {
    background: rgba(239,68,68,0.15);
    color: #fca5a5;
    border-color: rgba(239,68,68,0.3);
}

.sidebar-disconnect:disabled {
    opacity: 0.4;
    cursor: not-allowed;
}

/* Main Content */
.main-content {
    margin-left: var(--sidebar-width);
    flex: 1;
    min-height: 100vh;
    display: flex;
    flex-direction: column;
}

/* Header */
.admin-header {
    height: var(--header-height);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 24px;
    border-bottom: 1px solid var(--border-color);
    background: var(--bg-primary);
    position: sticky;
    top: 0;
    z-index: 50;
}

.header-title {
    font-size: 18px;
    font-weight: 600;
    letter-spacing: -0.01em;
}

.header-right { display: flex; align-items: center; gap: 16px; }

.connection-status {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    color: var(--text-secondary);
}

.connection-status.status-connected .status-dot { background: var(--success); }

.status-dot {
    width: 8px; height: 8px;
    border-radius: 50%;
    background: var(--danger);
}

.header-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px; height: 32px;
    border: 1px solid var(--border-color);
    border-radius: var(--radius);
    background: var(--bg-primary);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s;
}

.header-btn:hover {
    background: var(--bg-secondary);
    color: var(--text-primary);
}

/* Page Content */
.page-content {
    flex: 1;
    padding: 24px;
}

.page { max-width: 1400px; }

.page-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 20px;
    gap: 12px;
    flex-wrap: wrap;
}

.page-header-left, .page-header-right {
    display: flex;
    align-items: center;
    gap: 10px;
}

/* Stats Grid */
.stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 16px;
    margin-bottom: 24px;
}

.stats-card {
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    padding: 16px 20px;
    box-shadow: var(--shadow);
}

.stats-card-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
}

.stats-card-icon { display: flex; align-items: center; }

.stats-card-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.03em;
}

.stats-card-value {
    font-size: 28px;
    font-weight: 700;
    letter-spacing: -0.02em;
    line-height: 1.2;
}

.trend-up { color: var(--success); font-size: 12px; font-weight: 600; }
.trend-down { color: var(--danger); font-size: 12px; font-weight: 600; }

/* Panels */
.dashboard-panels {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 20px;
    margin-bottom: 24px;
}

@media (max-width: 900px) {
    .dashboard-panels { grid-template-columns: 1fr; }
}

.panel {
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    padding: 20px;
    box-shadow: var(--shadow);
    margin-bottom: 20px;
}

.panel-title {
    font-size: 15px;
    font-weight: 600;
    margin-bottom: 16px;
    letter-spacing: -0.01em;
}

.panel-header-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
}

.panel-header-row .panel-title { margin-bottom: 0; }

/* Badges */
.badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 9999px;
    font-size: 12px;
    font-weight: 500;
    line-height: 1.6;
}

.badge-success { background: rgba(34,197,94,0.12); color: var(--success); }
.badge-warning { background: rgba(245,158,11,0.12); color: var(--warning); }
.badge-danger { background: rgba(239,68,68,0.12); color: var(--danger); }
.badge-info { background: rgba(6,182,212,0.12); color: var(--info); }
.badge-neutral { background: rgba(100,116,139,0.12); color: var(--neutral); }

/* Buttons */
.btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 8px 16px;
    border: 1px solid transparent;
    border-radius: var(--radius);
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s;
    white-space: nowrap;
    text-decoration: none;
}

.btn:disabled { opacity: 0.5; cursor: not-allowed; }

.btn-primary {
    background: var(--accent);
    color: #ffffff;
    border-color: var(--accent);
}
.btn-primary:hover:not(:disabled) { background: var(--accent-hover); }

.btn-secondary {
    background: var(--bg-primary);
    color: var(--text-primary);
    border-color: var(--border-color);
}
.btn-secondary:hover:not(:disabled) { background: var(--bg-secondary); }

.btn-danger {
    background: var(--danger);
    color: #ffffff;
    border-color: var(--danger);
}
.btn-danger:hover:not(:disabled) { background: #dc2626; }

.btn-sm { padding: 4px 10px; font-size: 12px; }
.btn-block { width: 100%; }

/* Forms */
.form-group { margin-bottom: 16px; }

.form-label {
    display: block;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    margin-bottom: 6px;
}

.form-input {
    width: 100%;
    padding: 8px 12px;
    border: 1px solid var(--border-color);
    border-radius: var(--radius);
    background: var(--bg-primary);
    color: var(--text-primary);
    font-size: 14px;
    transition: border-color 0.15s;
    outline: none;
}

.form-input:focus { border-color: var(--accent); }

.form-input-half { max-width: 240px; }

.form-hint {
    display: block;
    font-size: 12px;
    color: var(--text-secondary);
    margin-top: 4px;
}

.form-error {
    padding: 8px 12px;
    background: rgba(239,68,68,0.08);
    border: 1px solid rgba(239,68,68,0.2);
    border-radius: var(--radius);
    color: var(--danger);
    font-size: 13px;
    margin-bottom: 12px;
}

.checkbox-group { display: flex; flex-wrap: wrap; gap: 12px; }

.checkbox-label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    cursor: pointer;
}

.modal-form .form-group:last-of-type { margin-bottom: 16px; }

.modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding-top: 8px;
}

/* Search */
.search-input {
    padding: 8px 12px;
    border: 1px solid var(--border-color);
    border-radius: var(--radius);
    background: var(--bg-primary);
    color: var(--text-primary);
    font-size: 14px;
    outline: none;
    width: 220px;
    transition: border-color 0.15s;
}

.search-input:focus { border-color: var(--accent); }
.search-input::placeholder { color: var(--text-secondary); }

/* Tables */
.table-wrapper {
    overflow-x: auto;
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    background: var(--bg-primary);
}

.data-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 14px;
}

.data-table thead { background: var(--bg-secondary); }

.data-table th {
    text-align: left;
    padding: 10px 14px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    white-space: nowrap;
    border-bottom: 1px solid var(--border-color);
}

.data-table td {
    padding: 10px 14px;
    border-bottom: 1px solid var(--border-color);
    vertical-align: middle;
}

.data-table tbody tr:hover { background: var(--bg-secondary); }
.data-table tbody tr:last-child td { border-bottom: none; }

.table-empty {
    text-align: center;
    padding: 32px 14px !important;
    color: var(--text-secondary);
}

.mono { font-family: 'SF Mono', 'Fira Code', 'Cascadia Code', monospace; font-size: 13px; }

.actions-cell { white-space: nowrap; }
.actions-cell .btn { margin-left: 4px; }

/* Banners */
.error-banner {
    padding: 10px 14px;
    background: rgba(239,68,68,0.08);
    border: 1px solid rgba(239,68,68,0.2);
    border-radius: var(--radius);
    color: var(--danger);
    font-size: 13px;
    margin-bottom: 16px;
}

.success-banner {
    padding: 10px 14px;
    background: rgba(34,197,94,0.08);
    border: 1px solid rgba(34,197,94,0.2);
    border-radius: var(--radius);
    color: var(--success);
    font-size: 13px;
    margin-bottom: 16px;
}

.loading {
    text-align: center;
    padding: 40px;
    color: var(--text-secondary);
    font-size: 14px;
}

/* Modal */
.modal-overlay {
    display: none;
    position: fixed;
    inset: 0;
    z-index: 200;
    align-items: center;
    justify-content: center;
}

.modal-overlay.modal-visible { display: flex; }

.modal-backdrop {
    position: absolute;
    inset: 0;
    background: rgba(0,0,0,0.5);
}

.modal {
    position: relative;
    background: var(--bg-primary);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
    width: 90%;
    max-width: 480px;
    max-height: 80vh;
    overflow-y: auto;
    padding: 0;
}

.modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-color);
}

.modal-title { font-size: 16px; font-weight: 600; }

.modal-close {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px; height: 28px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
}

.modal-close:hover { background: var(--bg-secondary); color: var(--text-primary); }

.modal-body { padding: 20px; }

/* Charts */
.chart-container { margin-bottom: 8px; }

.chart-title {
    font-size: 14px;
    font-weight: 600;
    margin-bottom: 12px;
}

.bar-chart {
    width: 100%;
    height: 200px;
    display: block;
}

.pie-chart-wrapper {
    display: flex;
    align-items: center;
    gap: 24px;
}

.pie-chart { width: 160px; height: 160px; flex-shrink: 0; }

.chart-legend { flex: 1; }

.legend-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 0;
    font-size: 13px;
}

.legend-color {
    width: 12px; height: 12px;
    border-radius: 3px;
    flex-shrink: 0;
}

.legend-label { color: var(--text-secondary); flex: 1; }
.legend-value { font-weight: 600; }

/* Progress Bar */
.progress-bar-container { margin-bottom: 16px; }

.progress-label {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    margin-bottom: 6px;
}

.progress-bar {
    height: 8px;
    background: var(--bg-secondary);
    border-radius: 9999px;
    overflow: hidden;
}

.progress-fill {
    height: 100%;
    border-radius: 9999px;
    transition: width 0.3s ease;
}

.progress-text {
    font-size: 12px;
    color: var(--text-secondary);
    margin-top: 4px;
    text-align: right;
}

/* Activity List */
.activity-list { display: flex; flex-direction: column; gap: 8px; }

.activity-item {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 8px 0;
    border-bottom: 1px solid var(--border-color);
}

.activity-item:last-child { border-bottom: none; }

.activity-main { display: flex; align-items: center; gap: 8px; }

.activity-action {
    font-weight: 600;
    font-size: 13px;
}

.activity-resource {
    font-family: 'SF Mono', monospace;
    font-size: 12px;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.activity-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--text-secondary);
}

.activity-time { margin-left: auto; }

/* Detail rows */
.detail-row {
    display: flex;
    align-items: flex-start;
    gap: 16px;
    padding: 10px 0;
    border-bottom: 1px solid var(--border-color);
}

.detail-row:last-child { border-bottom: none; }

.detail-label {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    min-width: 140px;
    flex-shrink: 0;
}

.detail-value {
    font-size: 14px;
    word-break: break-word;
}

.settings-grid { display: flex; flex-direction: column; }

/* Feature Grid */
.feature-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 8px;
}

.feature-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    background: var(--bg-secondary);
    border-radius: var(--radius);
    font-size: 13px;
}

.feature-name { font-weight: 500; }
.feature-enabled { color: var(--success); font-weight: 600; font-size: 12px; }
.feature-disabled { color: var(--text-secondary); font-size: 12px; }

/* Tabs */
.tab-bar {
    display: flex;
    gap: 0;
    border-bottom: 2px solid var(--border-color);
    margin-bottom: 16px;
}

.tab {
    padding: 8px 16px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    margin-bottom: -2px;
    transition: all 0.15s;
}

.tab:hover { color: var(--text-primary); }
.tab.tab-active { color: var(--accent); border-bottom-color: var(--accent); }

/* Pagination */
.pagination {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 16px;
    padding: 12px 0;
}

.pagination-info {
    font-size: 13px;
    color: var(--text-secondary);
}

.pagination-controls { display: flex; gap: 8px; }

/* Metrics */
.metrics-output {
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: var(--radius);
    padding: 16px;
    font-family: 'SF Mono', 'Fira Code', monospace;
    font-size: 12px;
    line-height: 1.6;
    overflow-x: auto;
    max-height: 400px;
    overflow-y: auto;
    white-space: pre-wrap;
    word-break: break-all;
}

/* Login Page */
.login-page {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    padding: 24px;
    background: var(--bg-secondary);
}

.login-card {
    background: var(--bg-primary);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
    padding: 40px;
    width: 100%;
    max-width: 420px;
}

.login-header {
    text-align: center;
    margin-bottom: 32px;
}

.login-title {
    font-size: 24px;
    font-weight: 700;
    margin: 12px 0 4px;
    letter-spacing: -0.02em;
}

.login-subtitle {
    font-size: 14px;
    color: var(--text-secondary);
}

.login-form .form-group { margin-bottom: 20px; }

.login-footer {
    margin-top: 24px;
    padding-top: 20px;
    border-top: 1px solid var(--border-color);
    text-align: center;
}

.login-footer p {
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.6;
}

.login-footer p:first-child { margin-bottom: 4px; }

/* Empty state */
.empty-state {
    text-align: center;
    padding: 40px;
    color: var(--text-secondary);
}
"#;

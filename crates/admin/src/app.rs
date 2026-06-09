use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;

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
                    <a href="#main-content" class="skip-nav" aria-label="Skip to main content">"Skip to main content"</a>
                    <div class="app-layout">
                        <Sidebar api=api/>
                        <main class="main-content" id="main-content" aria-label="Main content">
                            <Header api=api/>
                            <div class="page-content surface" aria-live="polite">
                                <Routes fallback=|| "Not found">
                                    <Route path=path!("/") view=move || {
                                        if connected {
                                            view! { <DashboardPage api=api/> }.into_any()
                                        } else {
                                            view! { <LoginPage api=api/> }.into_any()
                                        }
                                    }/>
                                    <Route path=path!("/login") view=move || view! { <LoginPage api=api/> }/>
                                    <Route path=path!("/users") view=move || view! { <UsersPage api=api/> }/>
                                    <Route path=path!("/storage") view=move || view! { <StoragePage api=api/> }/>
                                    <Route path=path!("/monitoring") view=move || view! { <MonitoringPage api=api/> }/>
                                    <Route path=path!("/settings") view=move || view! { <SettingsPage api=api/> }/>
                                    <Route path=path!("/federation") view=move || view! { <FederationPage api=api/> }/>
                                    <Route path=path!("/webhooks") view=move || view! { <WebhooksPage api=api/> }/>
                                    <Route path=path!("/audit") view=move || view! { <AuditPage api=api/> }/>
                                </Routes>
                            </div>
                        </main>
                    </div>
                }
            }}
        </Router>
    }
}

const GLOBAL_CSS: &str = r#"
:root {
    --bg-primary: #F5F0EB;
    --bg-secondary: #EDE8E3;
    --bg-sidebar: #1A1714;
    --bg-sidebar-hover: #2A2723;
    --text-primary: #2B2B2B;
    --text-secondary: #8B8178;
    --text-sidebar: #E8E0D8;
    --text-sidebar-muted: #8B8178;
    --border-color: #D4CBC0;
    --border-strong: #2B2B2B;
    --accent: #E85D04;
    --accent-dark: #D4520A;
    --accent-glow: rgba(232, 93, 4, 0.2);
    --crimson: #370617;
    --success: #16A34A;
    --warning: #CA8A04;
    --danger: #DC2626;
    --info: #E85D04;
    --neutral: #8B8178;
    --radius: 4px;
    --radius-lg: 12px;
    --shadow: 0 2px 4px rgba(0,0,0,0.1), 0 8px 16px rgba(0,0,0,0.08), inset 0 1px 0 rgba(255,255,255,0.5);
    --shadow-concrete: 0 2px 4px rgba(0,0,0,0.1), 0 8px 16px rgba(0,0,0,0.08), inset 0 1px 0 rgba(255,255,255,0.5);
    --shadow-iron: 0 4px 6px rgba(0,0,0,0.3), inset 0 1px 0 rgba(255,255,255,0.15);
    --shadow-emboss: inset 0 2px 4px rgba(0,0,0,0.15), inset 0 -1px 0 rgba(255,255,255,0.08);
    --sidebar-width: 240px;
    --header-height: 56px;
    --font-display: "IBM Plex Mono", "JetBrains Mono", "SF Mono", "Fira Code", ui-monospace, monospace;
    --font-body: "Inter", "IBM Plex Sans", system-ui, -apple-system, sans-serif;
}

@media (prefers-color-scheme: dark) {
    :root {
        --bg-primary: #1A1714;
        --bg-secondary: #221F1B;
        --bg-sidebar: #141210;
        --bg-sidebar-hover: #1A1714;
        --text-primary: #E8E0D8;
        --text-secondary: #9B9590;
        --text-sidebar: #E8E0D8;
        --text-sidebar-muted: #9B9590;
        --border-color: #3D3832;
        --border-strong: #E8E0D8;
        --shadow: 0 2px 4px rgba(0,0,0,0.3), 0 8px 16px rgba(0,0,0,0.2), inset 0 1px 0 rgba(255,255,255,0.05);
        --shadow-concrete: 0 2px 4px rgba(0,0,0,0.3), 0 8px 16px rgba(0,0,0,0.2), inset 0 1px 0 rgba(255,255,255,0.05);
        --shadow-iron: 0 4px 6px rgba(0,0,0,0.5), inset 0 1px 0 rgba(255,255,255,0.05);
        --shadow-emboss: inset 0 2px 4px rgba(0,0,0,0.3), inset 0 -1px 0 rgba(255,255,255,0.03);
    }
}

*, *::before, *::after { margin: 0; padding: 0; box-sizing: border-box; }

/* Skip navigation link */
.skip-nav {
    position: absolute;
    top: -100%;
    left: 0;
    z-index: 9999;
    padding: 12px 24px;
    background: var(--accent);
    color: #ffffff;
    font-family: var(--font-display);
    font-weight: 700;
    text-decoration: none;
    border: 3px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: 0 4px 0 var(--border-strong), 0 8px 16px rgba(0,0,0,0.15);
}
.skip-nav:focus {
    top: 8px;
    outline: 3px solid var(--accent);
    outline-offset: 2px;
}

/* Focus visible indicators — WCAG 2.1 AA */
:focus-visible {
    outline: 3px solid var(--accent);
    outline-offset: 2px;
}
button:focus-visible,
a:focus-visible,
input:focus-visible,
select:focus-visible {
    outline: 3px solid var(--accent);
    outline-offset: 2px;
    box-shadow: 0 0 0 4px var(--accent-glow);
}
input:focus-visible,
select:focus-visible {
    border-color: var(--accent);
}

/* Skip nav class for header title */
.sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0,0,0,0);
    border: 0;
}
.sr-only-focusable:focus {
    position: static;
    width: auto;
    height: auto;
    padding: inherit;
    margin: inherit;
    overflow: visible;
    clip: auto;
    white-space: nowrap;
}

body {
    font-family: var(--font-body);
    background: var(--bg-primary);
    background-image:
        radial-gradient(ellipse at 20% 50%, rgba(232,93,4,0.03) 0%, transparent 50%),
        radial-gradient(ellipse at 80% 20%, rgba(55,6,23,0.02) 0%, transparent 50%);
    color: var(--text-primary);
    line-height: 1.5;
    -webkit-font-smoothing: antialiased;
}

/* ── Surface layers (TD-024) ── */
.surface {
    background: linear-gradient(135deg, #F5F0EB 0%, #EDE8E3 100%);
    box-shadow: 0 2px 4px rgba(0,0,0,0.1), 0 8px 16px rgba(0,0,0,0.08), inset 0 1px 0 rgba(255,255,255,0.5);
}
.surface-dark {
    background: linear-gradient(135deg, #1A1714 0%, #221F1B 100%);
    box-shadow: 0 2px 4px rgba(0,0,0,0.3), 0 8px 16px rgba(0,0,0,0.2), inset 0 1px 0 rgba(255,255,255,0.05);
}
@media (prefers-color-scheme: dark) {
    .surface {
        background: linear-gradient(135deg, #1A1714 0%, #221F1B 100%);
        box-shadow: 0 2px 4px rgba(0,0,0,0.3), 0 8px 16px rgba(0,0,0,0.2), inset 0 1px 0 rgba(255,255,255,0.05);
    }
    .surface-dark {
        background: linear-gradient(135deg, #141210 0%, #1A1714 100%);
        box-shadow: 0 2px 4px rgba(0,0,0,0.4), 0 8px 16px rgba(0,0,0,0.3);
    }
}

/* ── Brutalist blocks (TD-024) ── */
.brutal-border { border: 3px solid var(--border-strong); }
.brutal-block {
    border: 3px solid var(--border-strong);
    box-shadow: 0 4px 0 var(--border-strong), 0 8px 16px rgba(0,0,0,0.15);
}

/* ── Monospace display font ── */
.font-display { font-family: var(--font-display); font-weight: 900; letter-spacing: -0.03em; }
.font-mono { font-family: var(--font-display); }
.mono { font-family: var(--font-display); font-size: 13px; }

.app-layout { display: flex; min-height: 100vh; }

/* Sidebar */
.sidebar {
    width: var(--sidebar-width);
    background: linear-gradient(135deg, #1A1714 0%, #221F1B 100%);
    box-shadow: 0 2px 4px rgba(0,0,0,0.3), 0 8px 16px rgba(0,0,0,0.2), inset 0 1px 0 rgba(255,255,255,0.05);
    color: var(--text-sidebar);
    position: fixed;
    top: 0; left: 0; bottom: 0;
    display: flex;
    flex-direction: column;
    z-index: 100;
    overflow-y: auto;
}

@media (prefers-color-scheme: dark) {
    .sidebar {
        background: linear-gradient(135deg, #141210 0%, #1A1714 100%);
        box-shadow: 0 2px 4px rgba(0,0,0,0.4), 0 8px 16px rgba(0,0,0,0.3);
    }
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
    font-weight: 900;
    letter-spacing: -0.03em;
    font-family: var(--font-display);
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
    font-weight: 600;
    font-family: var(--font-display);
    letter-spacing: 0.01em;
    transition: background 0.2s cubic-bezier(0.4, 0, 0.2, 1), color 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    cursor: pointer;
    margin-bottom: 2px;
}

.nav-item:hover {
    background: var(--bg-sidebar-hover);
    color: var(--text-sidebar);
}

.nav-item:focus-visible {
    outline: 3px solid var(--accent);
    outline-offset: -3px;
    box-shadow: 0 0 0 4px var(--accent-glow);
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
    font-family: var(--font-display);
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

.sidebar-disconnect:hover:not(:disabled) {
    background: rgba(220,38,38,0.15);
    color: #F87171;
    border-color: rgba(220,38,38,0.3);
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

/* Header — surface background + brutal border bottom */
.admin-header {
    height: var(--header-height);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 24px;
    border-bottom: 3px solid var(--border-strong);
    background: linear-gradient(135deg, #F5F0EB 0%, #EDE8E3 100%);
    box-shadow: 0 2px 4px rgba(0,0,0,0.1), 0 8px 16px rgba(0,0,0,0.08), inset 0 1px 0 rgba(255,255,255,0.5);
    position: sticky;
    top: 0;
    z-index: 50;
}

@media (prefers-color-scheme: dark) {
    .admin-header {
        border-bottom-color: #E8E0D8;
        background: linear-gradient(135deg, #1A1714 0%, #221F1B 100%);
        box-shadow: 0 2px 4px rgba(0,0,0,0.3), 0 8px 16px rgba(0,0,0,0.2), inset 0 1px 0 rgba(255,255,255,0.05);
    }
}

.header-title {
    font-size: 18px;
    font-weight: 900;
    letter-spacing: -0.03em;
    font-family: var(--font-display);
}

.header-right { display: flex; align-items: center; gap: 16px; }

.connection-status {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    color: var(--text-secondary);
    font-family: var(--font-display);
}

.connection-status.status-connected .status-dot { background: var(--accent); }

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
    border: 2px solid var(--border-color);
    border-radius: var(--radius);
    background: var(--bg-primary);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

.header-btn:hover {
    background: var(--bg-secondary);
    color: var(--text-primary);
    border-color: var(--accent);
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

/* Stats Grid — surface cards with shadow-concrete (TD-024) */
.stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 16px;
    margin-bottom: 24px;
}

.stats-card {
    background: linear-gradient(135deg, #F5F0EB 0%, #EDE8E3 100%);
    box-shadow: var(--shadow-concrete);
    border: none;
    border-radius: var(--radius-lg);
    padding: 16px 20px;
}

@media (prefers-color-scheme: dark) {
    .stats-card {
        background: linear-gradient(135deg, #1A1714 0%, #221F1B 100%);
        box-shadow: var(--shadow-concrete);
    }
}

.stats-card-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
}

.stats-card-icon { display: flex; align-items: center; }

.stats-card-title {
    font-size: 0.7rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-secondary);
    font-family: var(--font-display);
}

.stats-card-value {
    font-size: 28px;
    font-weight: 900;
    letter-spacing: -0.02em;
    line-height: 1.2;
    font-family: var(--font-display);
}

.trend-up { color: #16A34A; font-size: 12px; font-weight: 700; font-family: var(--font-display); }
.trend-down { color: #DC2626; font-size: 12px; font-weight: 700; font-family: var(--font-display); }

/* Panels — surface cards with shadow-concrete (TD-024) */
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
    background: linear-gradient(135deg, #F5F0EB 0%, #EDE8E3 100%);
    box-shadow: var(--shadow-concrete);
    border: none;
    border-radius: var(--radius-lg);
    padding: 20px;
    margin-bottom: 20px;
}

@media (prefers-color-scheme: dark) {
    .panel {
        background: linear-gradient(135deg, #1A1714 0%, #221F1B 100%);
        box-shadow: var(--shadow-concrete);
    }
}

/* Section headers — font-display */
.panel-title {
    font-size: 15px;
    font-weight: 800;
    margin-bottom: 16px;
    letter-spacing: -0.02em;
    font-family: var(--font-display);
}

.panel-header-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
}

.panel-header-row .panel-title { margin-bottom: 0; }

/* Badges — accent color for status indicators */
.badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 9999px;
    font-size: 12px;
    font-weight: 700;
    font-family: var(--font-display);
    line-height: 1.6;
}

.badge-success { background: rgba(22,163,74,0.12); color: #16A34A; }
.badge-warning { background: rgba(202,138,4,0.12); color: #CA8A04; }
.badge-danger { background: rgba(220,38,38,0.12); color: #DC2626; }
.badge-info { background: rgba(232,93,4,0.12); color: #E85D04; }
.badge-neutral { background: rgba(139,129,120,0.12); color: #8B8178; }

/* Buttons — brutal-block for primary, surface for secondary (TD-024) */
.btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 8px 16px;
    border: 2px solid transparent;
    border-radius: var(--radius);
    font-size: 14px;
    font-weight: 600;
    font-family: var(--font-display);
    letter-spacing: 0.01em;
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    white-space: nowrap;
    text-decoration: none;
}

.btn:focus-visible {
    outline: 3px solid var(--accent);
    outline-offset: 2px;
    box-shadow: 0 0 0 4px var(--accent-glow);
}

.btn:active:not(:disabled) {
    transform: translateY(1px);
}

.btn:disabled { opacity: 0.4; cursor: not-allowed; }

.btn-primary {
    background: var(--accent);
    color: #ffffff;
    border: 3px solid var(--border-strong);
    box-shadow: 0 4px 0 var(--border-strong), 0 8px 16px rgba(0,0,0,0.15);
}
.btn-primary:hover:not(:disabled) { background: var(--accent-dark); }

.btn-secondary {
    background: linear-gradient(135deg, #F5F0EB 0%, #EDE8E3 100%);
    color: var(--text-primary);
    border: 2px solid var(--border-color);
    box-shadow: var(--shadow-concrete);
}
.btn-secondary:hover:not(:disabled) {
    border-color: var(--accent);
    background: linear-gradient(135deg, #EDE8E3 0%, #E8E0D8 100%);
}

@media (prefers-color-scheme: dark) {
    .btn-secondary {
        background: linear-gradient(135deg, #1A1714 0%, #221F1B 100%);
        box-shadow: var(--shadow-concrete);
        border-color: #3D3832;
    }
    .btn-secondary:hover:not(:disabled) {
        background: linear-gradient(135deg, #221F1B 0%, #2A2723 100%);
        border-color: var(--accent);
    }
}

.btn-danger {
    background: var(--danger);
    color: #ffffff;
    border: 3px solid var(--border-strong);
    box-shadow: 0 4px 0 var(--border-strong), 0 8px 16px rgba(0,0,0,0.15);
}
.btn-danger:hover:not(:disabled) { background: #B91C1C; }

.btn-sm { padding: 4px 10px; font-size: 12px; }
.btn-block { width: 100%; }

/* Forms — labels use font-display (TD-024) */
.form-group { margin-bottom: 16px; }

.form-label {
    display: block;
    font-size: 0.7rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-secondary);
    font-family: var(--font-display);
    margin-bottom: 6px;
}

.form-input {
    width: 100%;
    padding: 8px 12px;
    border: 2px solid var(--border-color);
    border-radius: var(--radius);
    background: linear-gradient(135deg, #FAFAF8 0%, #F5F0EB 100%);
    color: var(--text-primary);
    font-size: 14px;
    font-family: var(--font-display);
    font-weight: 500;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    outline: none;
    box-shadow: inset 0 1px 2px rgba(0,0,0,0.06);
}

.form-input:focus,
.form-input:focus-visible {
    border-color: var(--accent);
    outline: 3px solid var(--accent);
    outline-offset: 2px;
    box-shadow: 0 0 0 4px var(--accent-glow), inset 0 1px 2px rgba(0,0,0,0.06);
}

@media (prefers-color-scheme: dark) {
    .form-input {
        border-color: #3D3832;
        color: #E8E0D8;
        background: linear-gradient(135deg, #221F1B 0%, #1A1714 100%);
        box-shadow: inset 0 1px 2px rgba(0,0,0,0.2);
    }
    .form-input:focus,
    .form-input:focus-visible {
        border-color: var(--accent);
        outline: 3px solid var(--accent);
        outline-offset: 2px;
        box-shadow: 0 0 0 4px var(--accent-glow), inset 0 1px 2px rgba(0,0,0,0.2);
    }
}

.form-input-half { max-width: 240px; }

.form-hint {
    display: block;
    font-size: 12px;
    color: var(--text-secondary);
    margin-top: 4px;
}

/* Error states — red-600 / red-50 */
.form-error {
    padding: 8px 12px;
    background: rgba(220,38,38,0.06);
    border-left: 4px solid #DC2626;
    border-radius: var(--radius);
    color: #B91C1C;
    font-size: 13px;
    font-family: var(--font-display);
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

/* Search (TD-024 + TD-023 focus) */
.search-input {
    padding: 8px 12px;
    border: 2px solid var(--border-color);
    border-radius: var(--radius);
    background: linear-gradient(135deg, #FAFAF8 0%, #F5F0EB 100%);
    color: var(--text-primary);
    font-size: 14px;
    font-family: var(--font-display);
    font-weight: 500;
    outline: none;
    width: 220px;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

.search-input:focus,
.search-input:focus-visible {
    border-color: var(--accent);
    outline: 3px solid var(--accent);
    outline-offset: 2px;
    box-shadow: 0 0 0 4px var(--accent-glow), inset 0 1px 2px rgba(0,0,0,0.06);
}

.search-input::placeholder { color: var(--text-secondary); }

@media (prefers-color-scheme: dark) {
    .search-input {
        border-color: #3D3832;
        color: #E8E0D8;
        background: linear-gradient(135deg, #221F1B 0%, #1A1714 100%);
    }
    .search-input:focus,
    .search-input:focus-visible {
        border-color: var(--accent);
        outline: 3px solid var(--accent);
        outline-offset: 2px;
        box-shadow: 0 0 0 4px var(--accent-glow), inset 0 1px 2px rgba(0,0,0,0.2);
    }
}

/* Tables — warm palette (TD-024 shadow) */
.table-wrapper {
    overflow-x: auto;
    border: none;
    border-radius: var(--radius-lg);
    background: var(--bg-primary);
    box-shadow: var(--shadow-concrete);
}

@media (prefers-color-scheme: dark) {
    .table-wrapper {
        background: var(--bg-primary);
        box-shadow: var(--shadow-concrete);
    }
}

.data-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 14px;
}

.data-table thead {
    background: var(--bg-primary);
    box-shadow: inset 0 2px 4px rgba(0,0,0,0.08), inset 0 -1px 0 rgba(255,255,255,0.3);
}

.data-table th {
    text-align: left;
    padding: 10px 14px;
    font-size: 0.7rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-secondary);
    font-family: var(--font-display);
    white-space: nowrap;
    border-bottom: none;
    position: sticky;
    top: 0;
    z-index: 5;
}

.data-table td {
    padding: 10px 14px;
    border-bottom: none;
    vertical-align: middle;
}

.data-table tbody tr {
    border-left: 3px solid transparent;
    transition: all 0.15s cubic-bezier(0.4, 0, 0.2, 1);
}

.data-table tbody tr:hover {
    background: rgba(232,93,4,0.04);
    border-left-color: var(--accent);
}
.data-table tbody tr:last-child td { border-bottom: none; }

.table-empty {
    text-align: center;
    padding: 32px 14px !important;
    color: var(--text-secondary);
}

.mono { font-family: var(--font-display); font-size: 13px; }

.actions-cell { white-space: nowrap; }
.actions-cell .btn { margin-left: 4px; }

/* Banners — error: red-600/red-50, success: green-600/green-50 */
.error-banner {
    padding: 10px 14px;
    background: rgba(220,38,38,0.06);
    border-left: 4px solid #DC2626;
    border-radius: var(--radius);
    color: #B91C1C;
    font-size: 13px;
    font-family: var(--font-display);
    margin-bottom: 16px;
}

.success-banner {
    padding: 10px 14px;
    background: rgba(22,163,74,0.06);
    border-left: 4px solid #16A34A;
    border-radius: var(--radius);
    color: #15803D;
    font-size: 13px;
    font-family: var(--font-display);
    margin-bottom: 16px;
}

.loading {
    text-align: center;
    padding: 40px;
    color: var(--text-secondary);
    font-size: 14px;
    font-family: var(--font-display);
}

/* Modal (TD-024 brutal-block) */
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
    background: linear-gradient(135deg, #F5F0EB 0%, #EDE8E3 100%);
    border: 3px solid var(--border-strong);
    border-radius: var(--radius-lg);
    box-shadow: 0 4px 0 var(--border-strong), 0 8px 16px rgba(0,0,0,0.15);
    width: 90%;
    max-width: 480px;
    max-height: 80vh;
    overflow-y: auto;
    padding: 0;
}

@media (prefers-color-scheme: dark) {
    .modal {
        background: linear-gradient(135deg, #1A1714 0%, #221F1B 100%);
        border-color: #E8E0D8;
        box-shadow: 0 4px 0 rgba(232,224,216,0.3), 0 8px 16px rgba(0,0,0,0.4);
    }
}

.modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 2px solid var(--border-color);
}

.modal-title {
    font-size: 16px;
    font-weight: 800;
    font-family: var(--font-display);
    letter-spacing: -0.02em;
}

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

.modal-close:focus-visible {
    outline: 3px solid var(--accent);
    outline-offset: 2px;
    box-shadow: 0 0 0 4px var(--accent-glow);
}

.modal-close:hover { background: var(--bg-secondary); color: var(--text-primary); }

.modal-body { padding: 20px; }

/* Charts */
.chart-container { margin-bottom: 8px; }

.chart-title {
    font-size: 14px;
    font-weight: 700;
    font-family: var(--font-display);
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
.legend-value { font-weight: 700; font-family: var(--font-display); }

/* Progress Bar */
.progress-bar-container { margin-bottom: 16px; }

.progress-label {
    font-size: 13px;
    font-weight: 700;
    color: var(--text-secondary);
    font-family: var(--font-display);
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
    font-weight: 700;
    font-size: 13px;
    font-family: var(--font-display);
}

.activity-resource {
    font-family: var(--font-display);
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
    font-size: 0.7rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-secondary);
    min-width: 140px;
    flex-shrink: 0;
    font-family: var(--font-display);
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

.feature-name { font-weight: 700; font-family: var(--font-display); }
.feature-enabled { color: #16A34A; font-weight: 700; font-size: 12px; font-family: var(--font-display); }
.feature-disabled { color: var(--text-secondary); font-size: 12px; font-family: var(--font-display); }

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
    font-weight: 600;
    font-family: var(--font-display);
    cursor: pointer;
    border-bottom: 2px solid transparent;
    margin-bottom: -2px;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

.tab:focus-visible {
    outline: 3px solid var(--accent);
    outline-offset: -3px;
    box-shadow: 0 0 0 4px var(--accent-glow);
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
    font-family: var(--font-display);
}

.pagination-controls { display: flex; gap: 8px; }

/* Metrics — monospace display font (TD-024) */
.metrics-output {
    background: var(--bg-secondary);
    border: 2px solid var(--border-color);
    border-radius: var(--radius);
    padding: 16px;
    font-family: var(--font-display);
    font-size: 12px;
    line-height: 1.6;
    overflow-x: auto;
    max-height: 400px;
    overflow-y: auto;
    white-space: pre-wrap;
    word-break: break-all;
}

/* Login Page (TD-024 brutal-block) */
.login-page {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    padding: 24px;
    background: var(--bg-secondary);
    background-image:
        radial-gradient(ellipse at 20% 50%, rgba(232,93,4,0.03) 0%, transparent 50%),
        radial-gradient(ellipse at 80% 20%, rgba(55,6,23,0.02) 0%, transparent 50%);
}

.login-card {
    background: linear-gradient(135deg, #F5F0EB 0%, #EDE8E3 100%);
    border: 3px solid var(--border-strong);
    border-radius: var(--radius-lg);
    box-shadow: 0 4px 0 var(--border-strong), 0 8px 16px rgba(0,0,0,0.15);
    padding: 40px;
    width: 100%;
    max-width: 420px;
}

@media (prefers-color-scheme: dark) {
    .login-card {
        background: linear-gradient(135deg, #1A1714 0%, #221F1B 100%);
        border-color: #E8E0D8;
        box-shadow: 0 4px 0 rgba(232,224,216,0.3), 0 8px 16px rgba(0,0,0,0.4);
    }
}

.login-header {
    text-align: center;
    margin-bottom: 32px;
}

.login-title {
    font-size: 24px;
    font-weight: 900;
    margin: 12px 0 4px;
    letter-spacing: -0.03em;
    font-family: var(--font-display);
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

.mt-2 { margin-top: 8px; }
.text-secondary-placeholder {
    color: var(--text-secondary);
    font-size: 13px;
    font-family: var(--font-display);
}
"#;

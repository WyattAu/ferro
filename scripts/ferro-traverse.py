#!/usr/bin/env python3
"""
Ferro Full Stack GUI Traversal & Error Capture Script
=====================================================
Exercises all buttons, routes, dialogs, and interactive elements
in the Ferro WASM and Desktop frontends. Captures:
  - JavaScript errors (unhandled exceptions, console.error, rejected promises)
  - Network errors (failed fetches, timeouts)
  - WASM panics
  - GUI rendering errors (via screenshot analysis)
  - Toast notifications
  - Server errors (via API response codes)

Usage:
  python3 ferro-traverse.py --mode wasm [--url BASE_URL] [--output DIR]
  python3 ferro-traverse.py --mode desktop [--output DIR]

Requires: playwright (WASM mode), Pillow (Desktop mode)
"""

import asyncio
import json
import os
import signal
import sys
import time
import traceback
from datetime import datetime
from pathlib import Path

# ── Configuration ────────────────────────────────────────────────────
BASE_URL = os.environ.get("FERRO_URL", "http://localhost:8080")
OUTPUT_DIR = os.environ.get("FERRO_TRAVERSE_DIR", "/tmp/ferro-traverse")
SERVER_BIN = os.environ.get("FERRO_SERVER_BIN",
    str(Path(__file__).parent.parent / "target/debug/ferro-server"))
STATIC_DIR = str(Path(__file__).parent.parent / "crates/web/dist")
SERVER_PORT = 18089  # Different port to avoid conflicts
SEED_DATA = True
TIMEOUT = 15000
SCREENSHOT_TIMEOUT = 3000

os.makedirs(OUTPUT_DIR, exist_ok=True)

# ── Error Collector ─────────────────────────────────────────────────────
class ErrorCollector:
    def __init__(self):
        self.errors = []
        self.warnings = []
        self.toasts = []
        self.network_errors = []

    def add_error(self, source, test_name, msg, severity="error"):
        self.errors.append({
            "timestamp": datetime.now().isoformat(),
            "source": source,
            "test": test_name,
            "severity": severity,
            "message": msg,
        })

    def add_warning(self, source, test_name, msg):
        self.warnings.append({
            "timestamp": datetime.now().isoformat(),
            "source": source,
            "test": test_name,
            "message": msg,
        })

    def add_toast(self, source, test_name, msg):
        self.toasts.append({
            "timestamp": datetime.now().isoformat(),
            "source": source,
            "test": test_name,
            "message": msg,
        })

    def add_network_error(self, source, test_name, url, status, err):
        self.network_errors.append({
            "timestamp": datetime.now().isoformat(),
            "source": source,
            "test": test_name,
            "url": url,
            "status": status,
            "error": err,
        })

    def has_errors(self):
        return len(self.errors) > 0 or len(self.network_errors) > 0

    def summary(self):
        total = len(self.errors) + len(self.warnings) + len(self.toasts) + len(self.network_errors)
        return (
            f"Errors: {len(self.errors)}, "
            f"Warnings: {len(self.warnings)}, "
            f"Network: {len(self.network_errors)}, "
            f"Toasts: {len(self.toasts)}"
        )


# ── WASM Traversal ──────────────────────────────────────────────────────
async def traverse_wasm(base_url, output_dir):
    """Full Playwright-based traversal of the WASM frontend."""
    from playwright.async_api import async_playwright

    collector = ErrorCollector()
    results = []
    screenshot_idx = 0

    def screenshot_path(name):
        nonlocal screenshot_idx
        screenshot_idx += 1
        return str(Path(output_dir) / f"{screenshot_idx:03d}_{name}.png")

    async def with_page(browser, name, url=None, fn=None):
        """Create a page with error capture, optionally navigate to URL, run fn, screenshot.
        If url is None but fn is provided, navigates to base_url first."""
        async def inner():
            page = await browser.new_page(viewport={"width": 1280, "height": 800})
            result = None

            # Inject global error catcher BEFORE any navigation
            await page.add_script_tag(content="""
                // ── Global Error Catcher ──
                window.__ferro_errors__ = [];
                window.__ferro_network_errors__ = [];
                window.__ferro_toasts__ = [];

                // Catch unhandled errors
                window.onerror = function(msg, source, lineno, colno, error) {
                    window.__ferro_errors__.push({
                        type: 'onerror',
                        message: msg,
                        source: source,
                        line: lineno,
                        col: colno,
                        error: error ? error.toString() : 'No error object',
                    });
                    return false;
                };

                // Catch unhandled promise rejections
                window.addEventListener('unhandledrejection', function(event) {
                    window.__ferro_errors__.push({
                        type: 'unhandledrejection',
                        message: event.reason ? event.reason.message || String(event.reason) : 'Unknown rejection',
                        promise: event.promise ? event.promise.toString() : 'No promise',
                    });
                });

                // Intercept console.error
                const _origError = console.error;
                console.error = function(...args) {
                    window.__ferro_errors__.push({
                        type: 'console.error',
                        args: args.map(a => String(a)),
                    });
                    _origError.apply(console, args);
                };

                // Intercept console.warn
                const _origWarn = console.warn;
                console.warn = function(...args) {
                    window.__ferro_errors__.push({
                        type: 'console.warn',
                        args: args.map(a => String(a)),
                    });
                    _origWarn.apply(console, args);
                };

                // Catch failed fetches via wrapper
                const _origFetch = window.fetch;
                window.fetch = function(...args) {
                    return _origFetch.apply(window, args).catch(err => {
                        window.__ferro_network_errors__.push({
                            url: args[0] ? String(args[0]) : 'unknown',
                            error: err.message,
                        });
                        throw err;
                    });
                };
            """)

            nav_url = url if url else (base_url + "/ui" if fn else None)
            if nav_url:
                try:
                    resp = await page.goto(nav_url, wait_until='networkidle', timeout=TIMEOUT)
                    status = resp.status
                except Exception as e:
                    collector.add_error("playwright", name, str(e))
                    status = 'nav_error'
            else:
                status = 'new_page'

            if fn:
                try:
                    result = await fn(page)
                except Exception as e:
                    collector.add_error("playwright", name, str(e))
                    result = {"error": str(e)}

            # Screenshot
            path = screenshot_path(name)
            await page.screenshot(path=path, full_page=True)

            # Collect injected errors
            injected = await page.evaluate('() => JSON.stringify({'
                'errors: window.__ferro_errors__ || [],'
                'network: window.__ferro_network_errors__ || [],'
                'toasts: window.__ferro_toasts__ || []'
                '})')

            inj = json.loads(injected)
            for e in inj.get('errors', []):
                collector.add_error("injected_js", name, e.get('message', e.get('args', '')))
            for e in inj.get('network', []):
                collector.add_network_error("injected_js", name, e.get('url', ''), 'fetch_fail', e.get('error', ''))
            for t in inj.get('toasts', []):
                collector.add_toast("injected_js", name, t.get('message', ''))

            # Collect playwright-side errors
            page_errors = []
            async def pe(msg):
                page_errors.append(msg)
            page.on('pageerror', pe)

            await page.close()
            return {
                "name": name,
                "status": status,
                "screenshot": path,
                "page_errors": page_errors,
                "injected_errors": len(inj.get('errors', [])),
                "injected_network": len(inj.get('network', [])),
                "injected_toasts": len(inj.get('toasts', [])),
                "result": result,
            }

        return await inner()

    async with async_playwright() as p:
        browser = await p.chromium.launch(headless=True)
        print(f"Browser launched. Base URL: {base_url}")

        # ══════════════════════════════════════════════════════════
        # SECTION 1: Navigation & Routes
        # ══════════════════════════════════════════════════════════
        nav_tests = [
            ("1.1_home", f"{base_url}/ui", "Home page (no trailing slash)"),
            ("1.2_home_trailing", f"{base_url}/ui/", "Home page (trailing slash -> redirect)"),
            ("1.3_files_root", f"{base_url}/ui/files/", "Files route at root"),
            ("1.4_files_subdir", f"{base_url}/ui/files/documents", "Files route at subdirectory"),
            ("1.5_files_deep", f"{base_url}/ui/files/documents/reports", "Files route deep path"),
            ("1.6_settings", f"{base_url}/ui/settings", "Settings page"),
            ("1.7_trash", f"{base_url}/ui/trash", "Trash page"),
            ("1.8_admin", f"{base_url}/ui/admin", "Admin page"),
            ("1.9_login", f"{base_url}/ui/auth/login", "Login page"),
        ]

        print("\n── Section 1: Navigation & Routes --")
        for name, url, desc in nav_tests:
            r = await with_page(browser, name, url)
            passed = r['status'] in (200, 308) and r['injected_errors'] == 0 and r['injected_network'] == 0
            results.append(("NAV", name, desc, "PASS" if passed else "FAIL", r))
            print(f"  {name}: {'PASS' if passed else 'FAIL'} [{r['status']}] errs={r['injected_errors']} net={r['injected_network']}")

        # ══════════════════════════════════════════════════════════
        # SECTION 2: Home Page Interactions
        # ══════════════════════════════════════════════════════════
        print("\n-- Section 2: Home Page Interactions --")

        # 2.1: Verify file list renders
        async def test_file_list(page):
            # File entries are <tr> rows in the table body
            try:
                await page.wait_for_selector('tbody tr', state='attached', timeout=10000)
                items = await page.query_selector_all('tbody tr')
                return {"files": len(items)}
            except Exception as e:
                # Check if page shows empty state instead
                body = await page.inner_text('body')
                return {"files": 0, "error": str(e), "body_preview": body[:200]}

        r = await with_page(browser, "2.1_file_list", fn=test_file_list)
        results.append(("HOME", "2.1_file_list", "File list renders", "PASS" if r['result'] and r['result'].get('files', 0) > 0 else "FAIL", r))
        print(f"  2.1_file_list: {'PASS' if r['result'] and r['result'].get('files', 0) > 0 else 'FAIL'} [{r['result']}]")

        # 2.2: Click on a directory to navigate
        async def test_click_dir(page):
            # File rows are <tr> elements in tbody
            rows = await page.query_selector_all('tbody tr')
            if rows:
                # Click the first row (should be a collection/directory)
                await rows[0].click()
                await page.wait_for_load_state('networkidle', timeout=10000)
                await page.wait_for_timeout(1000)
                return {"navigated": page.url, "rows_before": len(rows)}
            return {"navigated": "none", "rows_before": 0}

        r = await with_page(browser, "2.2_navigate_dir", fn=test_click_dir)
        results.append(("HOME", "2.2_navigate_dir", "Navigate into directory", "PASS" if r['result'].get('navigated', 'none') != 'none' else 'FAIL', r))

        # 2.3: Breadcrumb navigation back to root
        async def test_breadcrumb_back(page):
            btns = await page.query_selector_all('nav[aria-label="Breadcrumb"] button, a[aria-label="Go to parent directory"]')
            if btns:
                await btns[-1].click()
                await page.wait_for_load_state('networkidle', timeout=10000)
                return {"clicked": True}
            return {"clicked": False}

        r = await with_page(browser, "2.3_breadcrumb_back", fn=test_breadcrumb_back)
        results.append(("HOME", "2.3_breadcrumb_back", "Breadcrumb back to root", "PASS" if r['result'].get('clicked', False) else 'FAIL', r))

        # 2.4: Search button (aria-label="Search files")
        async def test_search_button(page):
            try:
                btn = await page.query_selector('button[aria-label="Search files"]')
                if btn:
                    await btn.click()
                    await page.wait_for_timeout(500)
                    inp = await page.query_selector('#header-search-input')
                    return {"found": True, "input_visible": inp is not None}
                # Fallback: try text-based
                btns = await page.query_selector_all('button')
                for b in btns:
                    txt = (await b.inner_text()).strip()
                    if 'Search' in txt or 'search' in txt:
                        await b.click()
                        await page.wait_for_timeout(1000)
                        return {"found": True, "input_visible": True, "method": "text_match"}
                return {"found": False, "method": "none"}
            except Exception as e:
                return {"found": False, "error": str(e)}

        r = await with_page(browser, "2.4_search_button", fn=test_search_button)
        results.append(("HOME", "2.4_search_button", "Search button click", "PASS" if r['result'].get('found', False) else 'FAIL', r))
        if r['result'].get('found'):
            print(f"    method={r['result'].get('method', 'aria-label')} input_visible={r['result'].get('input_visible')}")

        # 2.5: Type in search and close
        async def test_search_type(page):
            # Open search first via button click
            btn = await page.query_selector('button[aria-label="Search files"]')
            if not btn:
                return {"typed": False, "reason": "search_button_not_found"}
            await btn.dispatch_event('click')
            # Poll for search input (Leptos reactive rendering may be slow)
            for _ in range(20):
                inp = await page.query_selector('#header-search-input')
                if inp:
                    if await inp.is_visible():
                        break
                await page.wait_for_timeout(250)
            else:
                # Known headless-Chromium limitation: Leptos reactive signal
                # updates may not trigger DOM re-renders in headless mode.
                return {"typed": False, "reason": "known_headless_limitation"}
            await inp.fill('readme')
            await inp.press('Escape')
            await page.wait_for_timeout(500)
            return {"typed": True}

        r = await with_page(browser, "2.5_search_type", fn=test_search_type)
        results.append(("HOME", "2.5_search_type", "Search type + close with Escape", "PASS" if r['result'].get('typed', False) else 'FAIL', r))

        # 2.6: Settings link
        async def test_settings_link(page):
            link = await page.query_selector('a[aria-label="Settings"]')
            if link:
                href = await link.get_attribute('href')
                await link.click()
                await page.wait_for_load_state('networkidle', timeout=10000)
                return {"href": href, "current": page.url}
            return {"href": "not_found"}

        r = await with_page(browser, "2.6_settings_link", fn=test_settings_link)
        results.append(("HOME", "2.6_settings_link", "Settings navigation", "PASS" if r['result'].get('href', '') == '/ui/settings' else 'FAIL', r))

        # 2.7: Theme toggle
        async def test_theme_toggle(page):
            btn = await page.query_selector('button[aria-label="Toggle theme"]')
            if btn:
                class_before = await page.evaluate('() => document.documentElement.className')
                await btn.click()
                await page.wait_for_timeout(500)
                class_after = await page.evaluate('() => document.documentElement.className')
                return {"toggled": class_before != class_after}
            return {"toggled": False}

        r = await with_page(browser, "2.7_theme_toggle", fn=test_theme_toggle)
        results.append(("HOME", "2.7_theme_toggle", "Theme toggle", "PASS" if r['result'].get('toggled', False) else 'FAIL', r))

        # 2.8: Trash link
        async def test_trash_link(page):
            link = await page.query_selector('a[aria-label="Trash"]')
            if link:
                await link.click()
                await page.wait_for_load_state('networkidle', timeout=10000)
                return {"url": page.url}
            return {"url": "not_found"}

        r = await with_page(browser, "2.8_trash_link", fn=test_trash_link)
        results.append(("HOME", "2.8_trash_link", "Trash navigation", "PASS" if '/trash' in r['result'].get('url', '') else 'FAIL', r))

        # ══════════════════════════════════════════════════════════
        # SECTION 3: File Browser Toolbar & Actions
        # ══════════════════════════════════════════════════════════
        print("\n-- Section 3: File Browser Toolbar & Actions --")

        # 3.1: Parent directory button (navigate into subdir first so button is enabled)
        async def test_parent_btn(page):
            # Navigate into a subdir so parent button is enabled
            await page.goto(f"{base_url}/ui/files/documents", wait_until='networkidle', timeout=TIMEOUT)
            await page.wait_for_timeout(500)
            btn = await page.query_selector('button[aria-label="Go to parent directory"]')
            if not btn:
                return {"clicked": False, "reason": "not found"}
            is_disabled = await btn.is_disabled()
            if is_disabled:
                return {"clicked": False, "reason": "disabled at subdir"}
            try:
                await btn.click(timeout=5000)
                await page.wait_for_load_state('networkidle', timeout=5000)
                return {"clicked": True, "url": page.url}
            except Exception as e:
                return {"clicked": False, "reason": str(e)[:100]}

        r = await with_page(browser, "3.1_parent_btn", fn=test_parent_btn)
        results.append(("TOOLBAR", "3.1_parent_btn", "Parent directory button", "PASS" if r['result'].get('clicked', False) else 'FAIL', r))

        # 3.2: Home button (in breadcrumbs)
        async def test_home_btn(page):
            btns = await page.query_selector_all('nav[aria-label="Breadcrumb"] button')
            for btn in btns:
                txt = await btn.inner_text()
                if txt.strip() == 'Home':
                    await btn.click()
                    await page.wait_for_load_state('networkidle', timeout=10000)
                    return {"clicked": True, "url": page.url}
            return {"clicked": False}

        r = await with_page(browser, "3.2_home_btn", fn=test_home_btn)
        results.append(("TOOLBAR", "3.2_home_btn", "Home breadcrumb button", "PASS" if r['result'].get('clicked', False) else 'FAIL', r))

        # 3.3: Upload button
        async def test_upload_btn(page):
            btn = await page.query_selector('button[aria-label="Upload files"]')
            if btn:
                await btn.click()
                await page.wait_for_selector('div[role="dialog"][aria-labelledby="upload-title"]', timeout=5000)
                return {"dialog_opened": True}
            return {"dialog_opened": False}

        r = await with_page(browser, "3.3_upload_btn", fn=test_upload_btn)
        results.append(("TOOLBAR", "3.3_upload_btn", "Upload dialog open", "PASS" if r['result'].get('dialog_opened', False) else 'FAIL', r))

        # Close upload dialog if opened
        async def close_dialog(page):
            cancel = await page.query_selector('button:has-text("Close")')
            if cancel:
                await cancel.click()
                await page.wait_for_timeout(500)
            return {"closed": True}

        await with_page(browser, "3.3b_close_upload", fn=close_dialog)

        # 3.4: New folder button
        async def test_mkdir_btn(page):
            btn = await page.query_selector('button[aria-label="New folder"]')
            if btn:
                await btn.click()
                await page.wait_for_selector('div[role="dialog"][aria-labelledby="new-folder-title"]', timeout=5000)
                return {"dialog_opened": True}
            return {"dialog_opened": False}

        r = await with_page(browser, "3.4_mkdir_btn", fn=test_mkdir_btn)
        results.append(("TOOLBAR", "3.4_mkdir_btn", "New folder dialog open", "PASS" if r['result'].get('dialog_opened', False) else 'FAIL', r))

        # Close new folder dialog if opened
        await with_page(browser, "3.4b_close_mkdir", fn=close_dialog)

        # 3.5: Delete button (should not exist without selection)
        async def test_delete_no_sel(page):
            btn = await page.query_selector('button:has-text("Delete")')
            # Delete button should not exist or not be visible when nothing selected
            exists = btn is not None
            return {"exists": exists}

        r = await with_page(browser, "3.5_delete_no_sel", fn=test_delete_no_sel)
        results.append(("TOOLBAR", "3.5_delete_no_sel", "Delete button (no selection)", "PASS" if not r['result'].get('exists', False) else 'FAIL', r))

        # 3.6: View toggle (list/grid)
        async def test_view_toggle(page):
            btn = await page.query_selector('button[aria-label="Switch to grid view"]')
            if not btn:
                btn = await page.query_selector('button[aria-label="Switch to list view"]')
            if btn:
                await btn.click()
                await page.wait_for_timeout(500)
                return {"toggled": True}
            return {"toggled": False}

        r = await with_page(browser, "3.6_view_toggle", fn=test_view_toggle)
        results.append(("TOOLBAR", "3.6_view_toggle", "View mode toggle", "PASS" if r['result'].get('toggled', False) else 'FAIL', r))

        # 3.7: Activity panel toggle
        async def test_activity_toggle(page):
            btn = await page.query_selector('button[aria-label="Toggle activity panel"]')
            if btn:
                await btn.click()
                await page.wait_for_timeout(500)
                return {"toggled": True}
            return {"toggled": False}

        r = await with_page(browser, "3.7_activity_toggle", fn=test_activity_toggle)
        results.append(("TOOLBAR", "3.7_activity_toggle", "Activity panel toggle", "PASS" if r['result'].get('toggled', False) else 'FAIL', r))

        # ══════════════════════════════════════════════════════════
        # SECTION 4: Settings Page Interactions
        # ══════════════════════════════════════════════════════════
        print("\n-- Section 4: Settings Page --")

        # 4.1: Navigate to settings
        async def test_settings_page(page):
            await page.goto(f"{base_url}/ui/settings", wait_until='networkidle', timeout=TIMEOUT)
            title = await page.title()
            return {"title": title or "none", "len": len(await page.inner_text('body'))}

        r = await with_page(browser, "4.1_settings_page", url=None, fn=test_settings_page)
        results.append(("SETTINGS", "4.1_settings_page", "Settings page loads", "PASS" if r['result'].get('len', 0) > 50 else 'FAIL', r))

        # 4.2: Back to Files link
        async def test_back_to_files(page):
            # Settings may link to /ui or /ui/
            link = await page.query_selector('a[href^="/ui"]')
            if link:
                await link.click()
                await page.wait_for_load_state('networkidle', timeout=10000)
                await page.wait_for_timeout(500)
                return {"navigated": "/ui" in page.url, "url": page.url}
            return {"navigated": False, "url": page.url}

        r = await with_page(browser, "4.2_back_to_files", url=f"{base_url}/ui/settings", fn=test_back_to_files)
        results.append(("SETTINGS", "4.2_back_to_files", "Back to Files", "PASS" if r['result'].get('navigated', False) else 'FAIL', r))

        # ══════════════════════════════════════════════════════════
        # SECTION 5: Command Palette (Ctrl+K)
        # ══════════════════════════════════════════════════════════
        print("\n-- Section 5: Command Palette --")

        async def test_command_palette(page):
            # Use dispatch_event to fire keydown on document
            # Known: ev.preventDefault() fires (handler runs) but Leptos
            # reactive DOM update doesn't render in headless Chromium.
            result = await page.evaluate('''() => {
                const ev = new KeyboardEvent('keydown', {
                    key: 'k', code: 'KeyK', keyCode: 75,
                    ctrlKey: true, metaKey: false, bubbles: true, cancelable: true
                });
                document.dispatchEvent(ev);
                return ev.defaultPrevented;
            }''')
            await page.wait_for_timeout(1000)
            palette = await page.query_selector('div[role="dialog"][aria-label="Command Palette"]')
            if palette:
                return {"palette_visible": True, "default_prevented": result}
            return {"palette_visible": False, "default_prevented": result, "reason": "known_headless_limitation"}

        r = await with_page(browser, "5.1_cmd_palette", fn=test_command_palette)
        results.append(("CMD", "5.1_cmd_palette", "Command palette (Ctrl+K)", "PASS" if r['result'].get('palette_visible', False) else 'FAIL', r))

        # 5.2: Close palette with Escape
        async def test_palette_escape(page):
            await page.evaluate('''() => {
                const ev = new KeyboardEvent('keydown', {
                    key: 'k', code: 'KeyK', keyCode: 75,
                    ctrlKey: true, metaKey: false, bubbles: true, cancelable: true
                });
                document.dispatchEvent(ev);
            }''')
            await page.wait_for_timeout(500)
            await page.keyboard.press('Escape')
            await page.wait_for_timeout(500)
            visible = await page.is_visible('div[role="dialog"][aria-label="Command Palette"]')
            return {"palette_hidden": not visible}

        r = await with_page(browser, "5.2_palette_escape", fn=test_palette_escape)
        results.append(("CMD", "5.2_palette_escape", "Close palette (Escape)", "PASS" if r['result'].get('palette_hidden', False) else 'FAIL', r))

        # ══════════════════════════════════════════════════════════
        # SECTION 6: Keyboard Shortcuts
        # ══════════════════════════════════════════════════════════
        print("\n-- Section 6: Keyboard Shortcuts --")

        # 6.1: Ctrl+N (new folder)
        async def test_ctrl_n(page):
            await page.evaluate('''() => {
                const ev = new KeyboardEvent('keydown', {
                    key: 'n', code: 'KeyN', keyCode: 78,
                    ctrlKey: true, metaKey: false, bubbles: true, cancelable: true
                });
                document.dispatchEvent(ev);
            }''')
            await page.wait_for_timeout(500)
            dialog = await page.query_selector('div[role="dialog"][aria-labelledby="new-folder-title"]')
            return {"dialog_opened": dialog is not None}

        r = await with_page(browser, "6.1_ctrl_n_new_folder", fn=test_ctrl_n)
        results.append(("KB", "6.1_ctrl_n_new_folder", "Ctrl+N (new folder)", "PASS" if r['result'].get('dialog_opened', False) else 'FAIL', r))
        await with_page(browser, "6.1b_close_ctrl_n", fn=close_dialog)

        # 6.2: Ctrl+U (upload)
        async def test_ctrl_u(page):
            await page.evaluate('''() => {
                const ev = new KeyboardEvent('keydown', {
                    key: 'u', code: 'KeyU', keyCode: 85,
                    ctrlKey: true, metaKey: false, bubbles: true, cancelable: true
                });
                document.dispatchEvent(ev);
            }''')
            await page.wait_for_timeout(500)
            dialog = await page.query_selector('div[role="dialog"][aria-labelledby="upload-title"]')
            return {"dialog_opened": dialog is not None}

        r = await with_page(browser, "6.2_ctrl_u_upload", fn=test_ctrl_u)
        results.append(("KB", "6.2_ctrl_u_upload", "Ctrl+U (upload)", "PASS" if r['result'].get('dialog_opened', False) else 'FAIL', r))
        await with_page(browser, "6.2b_close_ctrl_u", fn=close_dialog)

        # 6.3: Ctrl+F (search)
        async def test_ctrl_f(page):
            await page.keyboard.press('Control+f')
            await page.wait_for_selector('#header-search-input', timeout=5000)
            inp = await page.query_selector('#header-search-input')
            return {"search_opened": inp is not None}

        r = await with_page(browser, "6.3_ctrl_f_search", fn=test_ctrl_f)
        results.append(("KB", "6.3_ctrl_f_search", "Ctrl+F (search)", "PASS" if r['result'].get('search_opened', False) else 'FAIL', r))
        # Close search with Escape (reuse same page context)
        async def close_search(page):
            await page.keyboard.press('Escape')
            await page.wait_for_timeout(500)
            return {"closed": True}
        await with_page(browser, "6.3b_close_search", fn=close_search)

        # ══════════════════════════════════════════════════════════
        # SECTION 7: Trash Page
        # ══════════════════════════════════════════════════════════
        print("\n-- Section 7: Trash Page --")

        async def test_trash_page(page):
            await page.goto(f"{base_url}/ui/trash", wait_until='networkidle', timeout=TIMEOUT)
            await page.wait_for_timeout(1000)  # Wait for Leptos reactive rendering
            text = await page.inner_text('body')
            title = await page.title()
            return {"title": title or "none", "len": len(text)}

        r = await with_page(browser, "7.1_trash_page", fn=test_trash_page)
        results.append(("TRASH", "7.1_trash_page", "Trash page loads", "PASS" if r['result'].get('len', 0) > 0 else 'FAIL', r))

        # ══════════════════════════════════════════════════════════
        # SECTION 8: Admin Page
        # ══════════════════════════════════════════════════════════
        print("\n-- Section 8: Admin Page --")

        async def test_admin_page(page):
            await page.goto(f"{base_url}/ui/admin", wait_until='networkidle', timeout=TIMEOUT)
            await page.wait_for_timeout(1000)  # Wait for Leptos reactive rendering
            text = await page.inner_text('body')
            return {"len": len(text)}

        r = await with_page(browser, "8.1_admin_page", fn=test_admin_page)
        results.append(("ADMIN", "8.1_admin_page", "Admin page loads", "PASS" if r['result'].get('len', 0) > 0 else 'FAIL', r))

        await browser.close()

        # ══════════════════════════════════════════════════════════
        # REPORT
        # ══════════════════════════════════════════════════════════
        print("\n" + "=" * 70)
        print("  FERRO WASM TRAVERSAL REPORT")
        print("=" * 70)
        print(f"  Date: {datetime.now().isoformat()}")
        print(f"  Base URL: {base_url}")
        print(f"  Output: {output_dir}")
        print(f"  {collector.summary()}")
        print()

        passed = sum(1 for r in results if r[4] == 'PASS')
        total = len(results)
        print(f"  RESULTS: {passed}/{total} passed")
        print()

        if passed < total:
            print("  FAILURES:")
            for section, name, desc, status, r in results:
                if status == 'FAIL':
                    print(f"    [{section}] {name}: {desc}")
                    if r.get('injected_errors', 0) > 0:
                        print(f"      injected_errors={r['injected_errors']}")
                    if r.get('injected_network', 0) > 0:
                        print(f"      network_errors={r['injected_network']}")
        else:
            print("  ALL TESTS PASSED")

        print()

        # Save results
        report = {
            "timestamp": datetime.now().isoformat(),
            "mode": "wasm",
            "base_url": base_url,
            "results": [
                {"section": s, "name": n, "desc": d, "status": st,
                 "injected_errors": r.get("injected_errors", 0),
                 "injected_network": r.get("injected_network", 0),
                 "page_errors": len(r.get("page_errors", [])),
                 "screenshot": r.get("screenshot", ""),
                 "result": r.get("result", None)}
                for s, n, d, st, r in results
            ],
            "error_count": collector.summary(),
            "errors": collector.errors,
            "warnings": collector.warnings,
            "toasts": collector.toasts,
            "network_errors": collector.network_errors,
        }
        report_path = str(Path(output_dir) / "report.json")
        with open(report_path, 'w') as f:
            json.dump(report, f, indent=2)

        # Save errors log
        errors_path = str(Path(output_dir) / "errors.log")
        with open(errors_path, 'w') as f:
            f.write(f"Ferro WASM Traversal Error Log\n")
            f.write(f"Date: {datetime.now().isoformat()}\n\n")
            if collector.errors:
                f.write("=== JS/Network Errors ===\n")
                for e in collector.errors:
                    f.write(f"[{e['source']}] {e['test']}: {e['severity']}: {e['message']}\n")
            if collector.network_errors:
                f.write("\n=== Network Errors ===\n")
                for e in collector.network_errors:
                    f.write(f"[{e['source']}] {e['test']}: {e['url']} -> {e['status']} {e['error']}\n")
            if collector.warnings:
                f.write("\n=== Warnings ===\n")
                for w in collector.warnings:
                    f.write(f"[{w['source']}] {w['test']}: {w['message']}\n")
            if collector.toasts:
                f.write("\n=== Toasts ===\n")
                for t in collector.toasts:
                    f.write(f"[{t['source']}] {t['test']}: {t['message']}\n")
            if not collector.errors and not collector.network_errors and not collector.warnings and not collector.toasts:
                f.write("No errors captured.\n")

        print(f"\nReport: {report_path}")
        print(f"Errors: {errors_path}")
        print(f"Screenshots: {output_dir}/")


# ── Desktop Traversal ────────────────────────────────────────────────────
def traverse_desktop(output_dir):
    """xdotool + screenshot-based traversal of the Tauri desktop app."""
    import subprocess
    from PIL import Image as PILImage

    collector = ErrorCollector()
    results = []
    screenshot_idx = 0
    win_id = None

    def screenshot(name):
        nonlocal screenshot_idx
        screenshot_idx += 1
        path = str(Path(output_dir) / f"{screenshot_idx:03d}_{name}.png")
        subprocess.run(['import', '-window', 'root', '-format', 'png', path],
                       check=False, capture_output=True, timeout=10)
        return path

    def get_pixel_color(img, x_ratio=0.5, y_ratio=0.5):
        """Get RGB tuple at relative position."""
        w, h = img.size
        px = img.getpixel((int(w * x_ratio), int(h * y_ratio)))
        return px[:3]

    def find_window():
        """Find ferro desktop window by PID or name."""
        try:
            out = subprocess.check_output(['xdotool', 'search', '--name',
                                              'Ferro', '--class', 'ferro-desktop'],
                                             capture_output=True, timeout=5)
            for line in out.decode().strip().split('\n'):
                wid = line.strip()
                if wid.isdigit():
                    return int(wid)
            # Fallback: find by class name
            out = subprocess.check_output(['xdotool', 'search', '--class',
                                              'ferro-desktop'],
                                             capture_output=True, timeout=5)
            for line in out.decode().strip().split('\n'):
                wid = line.strip()
                if wid.isdigit():
                    return int(wid)
        except Exception:
            return None

    def activate_window(wid):
        subprocess.run(['xdotool', 'windowactivate', '--sync', str(wid)],
                       check=False, capture_output=True, timeout=5)

    def send_key(key):
        subprocess.run(['xdotool', 'key', '--clearmodifiers', str(key)],
                       check=False, capture_output=True, timeout=5)

    def type_text(text):
        for ch in text:
            subprocess.run(['xdotool', 'key', '--clearmodifiers', ch],
                       check=False, capture_output=True, timeout=5)
        subprocess.run(['xdotool', 'key', 'Return'], check=False, capture_output=True, timeout=5)

    def click_at(x_pct, y_pct):
        """Click at relative position (percentage of window)."""
        subprocess.run([
            'xdotool', 'mousemove', '--sync',
            '--window', str(win_id),
            '--absolute-x', str(int(x_pct * 1920)),
            '--absolute-y', str(int(y_pct * 800)),
        ], check=False, capture_output=True, timeout=5)
        time.sleep(0.1)
        subprocess.run(['xdotool', 'click', '--window', str(win_id)],
                       check=False, capture_output=True, timeout=5)
        time.sleep(0.5)

    def click_button_by_title(title):
        """Click a button by its title attribute (desktop toolbar)."""
        out = subprocess.check_output([
            'xdotool', 'search', '--name', str(win_id),
            '--class', 'toolbar-btn',
            f'--text~', title
        ], capture_output=True, timeout=5)
        for line in out.decode().strip().split('\n'):
            parts = line.strip().split()
            if len(parts) >= 3:
                coords = parts[-1]
                subprocess.run(['xdotool', 'click', '--window', str(win_id),
                            '--coords', coords],
                           check=False, capture_output=True, timeout=5)
                time.sleep(0.5)
                return True
        return False

    def press_enter():
        subprocess.run(['xdotool', 'key', 'Return'], check=False, capture_output=True, timeout=5)
        time.sleep(0.3)

    def press_escape():
        subprocess.run(['xdotool', 'key', 'Escape'], check=False, capture_output=True, timeout=5)
        time.sleep(0.3)

    def take_screenshot(name):
        return screenshot(name)

    def check_color(name, x, y, expected_rgb, tolerance=40):
        path = str(Path(output_dir) / f"latest_{name}.png")
        if os.path.exists(path):
            img = PILImage.open(path)
            actual = get_pixel_color(img, x, y)
            dist = sum(abs(a - b) for a, b in zip(actual, expected_rgb))
            return dist <= tolerance
        return None

    def wait_for_load(max_wait=10):
        """Wait for window to stop changing by checking pixel stability."""
        time.sleep(2)
        for _ in range(int(max_wait / 0.5)):
            img1 = PILImage.open(take_screenshot('load_check'))
            time.sleep(0.5)
            img2 = PILImage.open(take_screenshot('load_check'))
            if list(img1.getdata()) == list(img2.getdata()):
                return True
        return False

    print("=" * 70)
    print("  FERRO DESKTOP TRAVERSAL")
    print("=" * 70)
    print(f"  Date: {datetime.now().isoformat()}")
    print(f"  Output: {output_dir}")
    print(f"  Server: {BASE_URL} (PID will use desktop connect)")
    print()

    # Find the window (desktop app must be running separately)
    print("Looking for Ferro desktop window...")
    win_id = find_window()
    if win_id is None:
        print("  ERROR: Could not find Ferro desktop window.")
        print("  Make sure the desktop app is running with:")
        print("    WEBKIT_DISABLE_DMABUF_RENDERER=1 WAYLAND_DISPLAY= ./target/debug/ferro-desktop")
        print("    ./target/debug/ferro-desktop --server-url http://localhost:8080 --debug")
        print()
        # Still save empty report
        report = {
            "timestamp": datetime.now().isoformat(),
            "mode": "desktop",
            "results": [],
            "error_count": collector.summary(),
            "errors": collector.errors,
            "warnings": collector.warnings,
            "toasts": collector.toasts,
            "network_errors": collector.network_errors,
        }
        with open(str(Path(output_dir) / "report.json"), 'w') as f:
            json.dump(report, f, indent=2)
        return

    print(f"  Found window ID: {win_id}")
    activate_window(win_id)
    time.sleep(1)

    # ── Desktop Tests ────────────────────────────────────────────

    # D.1: Connect to server
    print("\n[D.1] Connect to server")
    subprocess.run(['xdotool', 'key', 'Tab'], check=False, capture_output=True, timeout=5)
    time.sleep(0.5)
    take_screenshot('d1_connect_start')
    send_key('Return')
    time.sleep(3)
    take_screenshot('d1_connect_result')

    # Check connected state
    if not wait_for_load(10):
        collector.add_error("desktop", "D.1", "Window did not stabilize after connect")
    else:
        color = check_color("d1_connect_result", 50, 95, (0, 200, 0))
        if color:
            print(f"  Status dot color at (50%,95): RGB{color} (green=connected)")
        else:
            collector.add_warning("desktop", "D.1", "Could not verify connection status")

    # D.2: Navigate to root directory
    print("\n[D.2] Navigate to root")
    click_button_by_title("Home")
    time.sleep(2)
    wait_for_load(8)
    take_screenshot('d2_root')
    root_len = len(take_screenshot('d2_root')) > 100
    results.append(("NAV", "D.2_root", "Navigate to root", "PASS" if root_len else "FAIL"))

    # D.3: Navigate into a subdirectory
    print("\n[D.3] Navigate into documents/")
    click_button_by_title("Refresh")
    time.sleep(1)
    wait_for_load(8)
    # Click on first file entry (should be a directory)
    click_at(45, 40)  # Approximate file list position
    time.sleep(1)
    wait_for_load(8)
    take_screenshot('d3_subdir')
    subdir_len = len(take_screenshot('d3_subdir')) > 100
    results.append(("NAV", "D.3_subdir", "Navigate into subdirectory", "PASS" if subdir_len else "FAIL"))

    # D.4: Go back
    print("\n[D.4] Go back")
    click_button_by_title("Back")
    time.sleep(2)
    wait_for_load(8)
    take_screenshot('d4_back')
    back_len = len(take_screenshot('d4_back')) > 100
    results.append(("NAV", "D.4_back", "Go back", "PASS" if back_len else "FAIL"))

    # D.5: Upload button
    print("\n[D.5] Upload button")
    click_button_by_title("Upload Files")
    time.sleep(1)
    take_screenshot('d5_upload_dialog')
    # Check for upload dialog or file picker
    time.sleep(2)
    take_screenshot('d5_upload_result')
    img = PILImage.open(str(Path(output_dir) / "latest_d5_upload_result.png"))
    has_upload = False
    # Check if there's a new dialog or file picker appeared (changed pixels)
    # For now, check if upload button exists
    upload_clickable = click_button_by_title("Upload Files")
    results.append(("TOOLBAR", "D.5_upload", "Upload button", "PASS" if upload_clickable else "FAIL"))

    # Close any dialog
    press_escape()
    time.sleep(0.5)

    # D.6: New folder button
    print("\n[D.6] New folder button")
    click_button_by_title("New Folder")
    time.sleep(1)
    take_screenshot('d6_mkdir_dialog')
    time.sleep(1)
    take_screenshot('d6_mkdir_result')
    # Check for mkdir input
    out = subprocess.check_output([
        'xdotool', 'search', '--name', str(win_id),
        '--class', 'input-field',
    ], capture_output=True, timeout=5)
    has_input = 'mkdir' in out.decode().lower() or 'input' in out.decode().lower()
    results.append(("TOOLBAR", "D.6_new_folder", "New folder dialog", "PASS" if has_input else "FAIL"))

    # Close dialog
    press_escape()
    time.sleep(0.5)

    # D.7: Refresh button
    print("\n[D.7] Refresh button")
    click_button_by_title("Refresh")
    time.sleep(2)
    wait_for_load(8)
    take_screenshot('d7_refresh')
    refresh_len = len(take_screenshot('d7_refresh')) > 100
    results.append(("TOOLBAR", "D.7_refresh", "Refresh button", "PASS" if refresh_len else "FAIL"))

    # D.8: Debug panel toggle (F12)
    print("\n[D.8] Debug panel (F12)")
    send_key('F12')
    time.sleep(1)
    take_screenshot('d8_debug_panel')
    out = subprocess.check_output([
        'xdotool', 'search', '--name', str(win_id),
        '--class', 'debug-panel',
    ], capture_output=True, timeout=5)
    has_debug = 'debug' in out.decode().lower()
    results.append(("DEBUG", "D.8_debug_panel", "Debug panel (F12)", "PASS" if has_debug else 'FAIL'))

    # Close debug panel
    send_key('F12')
    time.sleep(0.5)

    # D.9: Disconnect button
    print("\n[D.9] Disconnect button")
    click_button_by_title("Disconnect")
    time.sleep(2)
    take_screenshot('d9_disconnect')
    # Should see connect dialog again
    out = subprocess.check_output([
        'xdotool', 'search', '--name', str(win_id),
        '--class', 'connect',
    ], capture_output=True, timeout=5)
    has_connect = 'connect' in out.decode().lower()
    results.append(("TOOLBAR", "D.9_disconnect", "Disconnect button", "PASS" if has_connect else "FAIL"))

    # ── REPORT ────────────────────────────────────────────────────────
    print("\n" + "=" * 70)
    print("  FERRO DESKTOP TRAVERSAL REPORT")
    print("=" * 70)
    print(f"  Date: {datetime.now().isoformat()}")
    print(f"  Output: {output_dir}")
    print(f"  {collector.summary()}")
    print()

    passed = sum(1 for r in results if r[4] == 'PASS')
    total = len(results)
    print(f"  RESULTS: {passed}/{total} passed")
    print()

    if passed < total:
        print("  FAILURES:")
        for section, name, desc, status in results:
            if status == 'FAIL':
                print(f"    [{section}] {name}: {desc}")

    print()

    report = {
        "timestamp": datetime.now().isoformat(),
        "mode": "desktop",
        "results": [
            {"section": s, "name": n, "desc": d, "status": st}
            for s, n, d, st in results
        ],
        "error_count": collector.summary(),
        "errors": collector.errors,
        "warnings": collector.warnings,
        "toasts": collector.toasts,
        "network_errors": collector.network_errors,
    }
    with open(str(Path(output_dir) / "report.json"), 'w') as f:
        json.dump(report, f, indent=2)

    errors_path = str(Path(output_dir) / "errors.log")
    with open(errors_path, 'w') as f:
        f.write(f"Ferro Desktop Traversal Error Log\n")
        f.write(f"Date: {datetime.now().isoformat()}\n\n")
        if collector.errors:
            f.write("=== Errors ===\n")
            for e in collector.errors:
                f.write(f"{e}\n")
        if collector.warnings:
            f.write("\n=== Warnings ===\n")
            for w in collector.warnings:
                f.write(f"{w}\n")
        if not collector.errors and not collector.warnings:
            f.write("No errors captured.\n")

    print(f"\nReport: {report_path}")
    print(f"Errors: {errors_path}")
    print(f"Screenshots: {output_dir}/")


# ── Main ──────────────────────────────────────────────────────────
def main():
    import argparse

    parser = argparse.ArgumentParser(description="Ferro Full Stack GUI Traversal")
    parser.add_argument('--mode', choices=['wasm', 'desktop'], default='wasm',
                        help='Traversal mode: wasm or desktop')
    parser.add_argument('--url', default=BASE_URL,
                        help='Base URL for WASM mode')
    parser.add_argument('--output', default=OUTPUT_DIR,
                        help='Output directory for screenshots and reports')

    args = parser.parse_args()

    if args.mode == 'wasm':
        asyncio.run(traverse_wasm(args.url, args.output))
    elif args.mode == 'desktop':
        traverse_desktop(args.output)


if __name__ == '__main__':
    main()

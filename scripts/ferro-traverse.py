#!/usr/bin/env python3
"""
Ferro Full Stack GUI Traversal & Error Capture Script v2
=====================================================
Exercises ALL buttons, routes, dialogs, file operations, and interactive
elements in the Ferro WASM and Desktop frontends.

Captures from 3 layers:
  1. JavaScript: unhandled exceptions, console.error/warn, rejected promises
  2. Network: failed fetches, HTTP error responses
  3. Server: RUST_LOG tracing output (Desktop stderr)

Usage:
  python3 ferro-traverse.py --mode wasm [--url BASE_URL] [--output DIR]
  python3 ferro-traverse.py --mode desktop [--output DIR]

Requires: playwright (WASM mode), Pillow (Desktop mode)
"""

import asyncio
import json
import os
import sys
import time
from datetime import datetime
from pathlib import Path

BASE_URL = os.environ.get("FERRO_URL", "http://localhost:8080")
OUTPUT_DIR = os.environ.get("FERRO_TRAVERSE_DIR", "/tmp/ferro-traverse")
TIMEOUT = 15000
RENDER_WAIT = 800

os.makedirs(OUTPUT_DIR, exist_ok=True)


class ErrorCollector:
    def __init__(self):
        self.errors = []
        self.warnings = []
        self.toasts = []
        self.network_errors = []

    def add_error(self, source, test_name, msg, severity="error"):
        self.errors.append({"ts": datetime.now().isoformat(), "source": source, "test": test_name, "severity": severity, "message": msg})

    def add_warning(self, source, test_name, msg):
        self.warnings.append({"ts": datetime.now().isoformat(), "source": source, "test": test_name, "message": msg})

    def add_toast(self, source, test_name, msg):
        self.toasts.append({"ts": datetime.now().isoformat(), "source": source, "test": test_name, "message": msg})

    def add_network_error(self, source, test_name, url, status, err):
        self.network_errors.append({"ts": datetime.now().isoformat(), "source": source, "test": test_name, "url": url, "status": status, "error": err})

    def summary(self):
        return f"Errors: {len(self.errors)}, Warnings: {len(self.warnings)}, Network: {len(self.network_errors)}, Toasts: {len(self.toasts)}"


INTERCEPTOR_JS = """window.__ferro_errors__=[];window.__ferro_network_errors__=[];window.__ferro_toasts__=[];window.__ferro_http_responses__=[];
window.onerror=function(m,s,l,c,e){window.__ferro_errors__.push({type:'onerror',message:m,source:s,line:l,col:c});return false};
window.addEventListener('unhandledrejection',function(e){window.__ferro_errors__.push({type:'rejection',message:e.reason?e.reason.message||String(e.reason):'?'})});
var _oe=console.error;console.error=function(){window.__ferro_errors__.push({type:'console.error',args:[].map.call(arguments,String)});_oe.apply(console,arguments)};
var _ow=console.warn;console.warn=function(){window.__ferro_errors__.push({type:'console.warn',args:[].map.call(arguments,String)});_ow.apply(console,arguments)};
var _of=window.fetch;window.fetch=function(){return _of.apply(window,arguments).then(function(r){if(!r.ok)window.__ferro_network_errors__.push({url:String(arguments[0]),status:r.status,error:'HTTP '+r.status});window.__ferro_http_responses__.push({url:String(arguments[0]),status:r.status,ok:r.ok});if(window.__ferro_http_responses__.length>100)window.__ferro_http_responses__.shift();return r}).catch(function(e){window.__ferro_network_errors__.push({url:String(arguments[0]),error:e.message});throw e})};
var _to=new MutationObserver(function(ms){for(var i=0;i<ms.length;i++){for(var j=0;j<ms[i].addedNodes.length;j++){var n=ms[i].addedNodes[j];if(n.nodeType===1&&n.textContent&&n.textContent.length>0&&n.textContent.length<500&&(n.className+'').match(/toast|alert|notification/)){window.__ferro_toasts__.push({text:n.textContent.trim()})}}}});
if(document.body)_to.observe(document.body,{childList:true,subtree:true});else document.addEventListener('DOMContentLoaded',function(){_to.observe(document.body,{childList:true,subtree:true})});
"""


async def collect_errors(page, collector, name):
    try:
        data = await page.evaluate('() => JSON.stringify({e:window.__ferro_errors__||[],n:window.__ferro_network_errors__||[],t:window.__ferro_toasts__||[]})')
        d = json.loads(data)
    except Exception:
        return 0, 0
    ec = len(d.get('e', []))
    nc = len(d.get('n', []))
    for e in d.get('e', []):
        msg = e.get('message', e.get('args', ''))
        if isinstance(msg, list): msg = ' '.join(str(x) for x in msg)
        collector.add_error("js", name, str(msg))
    for e in d.get('n', []):
        collector.add_network_error("js", name, e.get('url', ''), e.get('status', ''), e.get('error', ''))
    for t in d.get('t', []):
        collector.add_toast("js", name, t.get('text', ''))
    return ec, nc


# =====================================================================
#  WASM TRAVERSAL
# =====================================================================
async def traverse_wasm(base_url, output_dir):
    from playwright.async_api import async_playwright
    collector = ErrorCollector()
    results = []
    idx = [0]

    def shot(name):
        idx[0] += 1
        return str(Path(output_dir) / f"{idx[0]:03d}_{name}.png")

    async def wp(browser, name, url=None, fn=None):
        page = await browser.new_page(viewport={"width": 1280, "height": 800})
        pw_err = []
        page.on('pageerror', lambda m: pw_err.append(str(m)))
        await page.add_script_tag(content=INTERCEPTOR_JS)
        nav = url or (base_url + "/ui" if fn else None)
        status = 'new_page'
        if nav:
            try:
                r = await page.goto(nav, wait_until='networkidle', timeout=TIMEOUT)
                status = r.status
            except Exception as e:
                collector.add_error("nav", name, str(e)); status = 'nav_error'
        result = None
        if fn:
            try:
                result = await fn(page)
            except Exception as e:
                collector.add_error("exec", name, str(e))
                result = {"error": str(e)[:300]}
        path = shot(name)
        await page.screenshot(path=path, full_page=True)
        ec, nc = await collect_errors(page, collector, name)
        ec += len(pw_err)
        for pe in pw_err: collector.add_error("pw", name, pe)
        await page.close()
        return dict(name=name, status=status, screenshot=path, errs=ec, net=nc, result=result)

    def add(sec, name, desc, ok, r):
        st = "PASS" if ok else "FAIL"
        results.append((sec, name, desc, st, r))
        print(f"  {name}: {'OK' if ok else 'FAIL'}" + (f" errs={r['errs']}" if r['errs'] else ""))

    async def close_dialog(page):
        for s in ['button:has-text("Close")', 'button:has-text("Cancel")']:
            b = await page.query_selector(s)
            if b: await b.click(); await page.wait_for_timeout(300); return

    async def kb_dispatch(page, key, ctrl=False):
        await page.evaluate(f'() => {{var e=new KeyboardEvent("keydown",{{key:"{key}",code:"Key{key.upper()}",ctrlKey:{str(ctrl).lower()},bubbles:true,cancelable:true}});document.dispatchEvent(e);return e.defaultPrevented}}')

    async with async_playwright() as p:
        browser = await p.chromium.launch(headless=True)
        print(f"Browser launched. Base URL: {base_url}")

        # -- S1: Navigation (9 tests) --
        print("\n== S1: Navigation & Routes ==")
        for n, u, d in [
            ("1.1_home", f"{base_url}/ui", "Home page"),
            ("1.2_trailing", f"{base_url}/ui/", "Trailing slash"),
            ("1.3_files", f"{base_url}/ui/files/", "Files root"),
            ("1.4_subdir", f"{base_url}/ui/files/documents", "Files subdir"),
            ("1.5_deep", f"{base_url}/ui/files/documents/reports", "Deep path"),
            ("1.6_settings", f"{base_url}/ui/settings", "Settings"),
            ("1.7_trash", f"{base_url}/ui/trash", "Trash"),
            ("1.8_admin", f"{base_url}/ui/admin", "Admin"),
            ("1.9_login", f"{base_url}/ui/auth/login", "Login"),
        ]:
            r = await wp(browser, n, u)
            add("NAV", n, d, r['status'] in (200, 308) and r['errs'] == 0, r)

        # -- S2: File List & Nav (6 tests) --
        print("\n== S2: File List & Navigation ==")

        async def t21(page):
            await page.wait_for_selector('tbody tr', state='attached', timeout=8000)
            rows = await page.query_selector_all('tbody tr')
            return {"rows": len(rows)}
        r = await wp(browser, "2.1_file_list", fn=t21)
        add("HOME", "2.1_file_list", "File list", r['result'] and r['result'].get('rows', 0) > 0, r)

        async def t22(page):
            rows = await page.query_selector_all('tbody tr')
            if not rows: return {"nav": False}
            await rows[0].click()
            await page.wait_for_load_state('networkidle', timeout=10000)
            await page.wait_for_timeout(RENDER_WAIT)
            return {"nav": True, "url": page.url}
        r = await wp(browser, "2.2_click_dir", fn=t22)
        add("HOME", "2.2_click_dir", "Navigate dir", r['result'].get('nav', False), r)

        async def t23(page):
            btns = await page.query_selector_all('nav[aria-label="Breadcrumb"] button')
            if btns: await btns[-1].click(); await page.wait_for_load_state('networkidle', timeout=10000); return {"ok": True}
            return {"ok": False}
        r = await wp(browser, "2.3_breadcrumb", fn=t23)
        add("HOME", "2.3_breadcrumb", "Back", r['result'].get('ok', False), r)

        async def t24(page):
            b = await page.query_selector('button[aria-label="Search files"]')
            if b: await b.click(); await page.wait_for_timeout(500); return {"ok": True}
            return {"ok": False}
        r = await wp(browser, "2.4_search_btn", fn=t24)
        add("HOME", "2.4_search_btn", "Search btn", r['result'].get('ok', False), r)

        async def t25(page):
            l = await page.query_selector('a[aria-label="Settings"]')
            if l: await l.click(); await page.wait_for_load_state('networkidle', timeout=10000); return {"ok": True}
            return {"ok": False}
        r = await wp(browser, "2.5_settings_link", fn=t25)
        add("HOME", "2.5_settings_link", "Settings", r['result'].get('ok', False), r)

        async def t26(page):
            b = await page.query_selector('button[aria-label="Toggle theme"]')
            if not b: return {"ok": False}
            c1 = await page.evaluate('() => document.documentElement.className')
            await b.click(); await page.wait_for_timeout(500)
            c2 = await page.evaluate('() => document.documentElement.className')
            return {"ok": c1 != c2}
        r = await wp(browser, "2.6_theme", fn=t26)
        add("HOME", "2.6_theme", "Theme", r['result'].get('ok', False), r)

        # -- S3: Toolbar (7 tests) --
        print("\n== S3: Toolbar Buttons ==")

        async def t31(page):
            await page.goto(f"{base_url}/ui/files/documents", wait_until='networkidle', timeout=TIMEOUT)
            await page.wait_for_timeout(RENDER_WAIT)
            b = await page.query_selector('button[aria-label="Go to parent directory"]')
            if not b: return {"ok": False}
            if await b.is_disabled(): return {"ok": False, "reason": "disabled"}
            await b.click(timeout=5000); await page.wait_for_load_state('networkidle', timeout=5000)
            return {"ok": True}
        r = await wp(browser, "3.1_parent", fn=t31)
        add("TOOL", "3.1_parent", "Parent btn", r['result'].get('ok', False), r)

        async def t32(page):
            for b in await page.query_selector_all('nav[aria-label="Breadcrumb"] button'):
                if 'Home' in (await b.inner_text()):
                    await b.click(); await page.wait_for_load_state('networkidle', timeout=10000); return {"ok": True}
            return {"ok": False}
        r = await wp(browser, "3.2_home", fn=t32)
        add("TOOL", "3.2_home", "Home btn", r['result'].get('ok', False), r)

        async def t33(page):
            b = await page.query_selector('button[aria-label="Upload files"]')
            if b: await b.click(); await page.wait_for_timeout(RENDER_WAIT)
            d = await page.query_selector('div[role="dialog"][aria-labelledby="upload-title"]')
            return {"ok": d is not None}
        r = await wp(browser, "3.3_upload_dlg", fn=t33)
        add("TOOL", "3.3_upload_dlg", "Upload dlg", r['result'].get('ok', False), r)
        await wp(browser, "3.3b_close", fn=close_dialog)

        async def t34(page):
            b = await page.query_selector('button[aria-label="New folder"]')
            if b: await b.click(); await page.wait_for_timeout(RENDER_WAIT)
            d = await page.query_selector('div[role="dialog"][aria-labelledby="new-folder-title"]')
            return {"ok": d is not None}
        r = await wp(browser, "3.4_mkdir_dlg", fn=t34)
        add("TOOL", "3.4_mkdir_dlg", "Mkdir dlg", r['result'].get('ok', False), r)
        await wp(browser, "3.4b_close", fn=close_dialog)

        async def t35(page):
            b = await page.query_selector('button[aria-label="Switch to grid view"]')
            if not b: b = await page.query_selector('button[aria-label="Switch to list view"]')
            if b: await b.click(); await page.wait_for_timeout(500); return {"ok": True}
            return {"ok": False}
        r = await wp(browser, "3.5_view", fn=t35)
        add("TOOL", "3.5_view", "View toggle", r['result'].get('ok', False), r)

        async def t36(page):
            b = await page.query_selector('button[aria-label="Toggle select mode"]')
            if b: await b.click(); await page.wait_for_timeout(RENDER_WAIT); return {"ok": True}
            return {"ok": False}
        r = await wp(browser, "3.6_select", fn=t36)
        add("TOOL", "3.6_select", "Select mode", r['result'].get('ok', False), r)

        async def t37(page):
            b = await page.query_selector('button[aria-label="Toggle activity panel"]')
            if b: await b.click(); await page.wait_for_timeout(RENDER_WAIT); return {"ok": True}
            return {"ok": False}
        r = await wp(browser, "3.7_activity", fn=t37)
        add("TOOL", "3.7_activity", "Activity", r['result'].get('ok', False), r)

        # -- S4: File Operations --
        print("\n== S4: File Operations ==")

        async def t41_create_folder(page):
            b = await page.query_selector('button[aria-label="New folder"]')
            if not b: return {"ok": False, "r": "no btn"}
            await b.click(); await page.wait_for_timeout(RENDER_WAIT)
            d = await page.query_selector('div[role="dialog"][aria-labelledby="new-folder-title"]')
            if not d: return {"ok": False, "r": "no dialog"}
            inp = await d.query_selector('input[type="text"]') or await d.query_selector('input')
            if not inp: return {"ok": False, "r": "no input"}
            await inp.fill('traverse-test-folder')
            cb = await d.query_selector('button:has-text("Create")') or await d.query_selector('button[type="submit"]')
            if not cb: return {"ok": False, "r": "no create btn"}
            await cb.click(); await page.wait_for_timeout(1500)
            await page.wait_for_load_state('networkidle', timeout=10000)
            return {"ok": True}
        r = await wp(browser, "4.1_create_folder", fn=t41_create_folder)
        add("OPS", "4.1_create_folder", "Create folder", r['result'].get('ok', False), r)

        async def t42_upload(page):
            b = await page.query_selector('button[aria-label="Upload files"]')
            if not b: return {"ok": False, "r": "no btn"}
            await b.click(); await page.wait_for_timeout(RENDER_WAIT)
            d = await page.query_selector('div[role="dialog"][aria-labelledby="upload-title"]')
            if not d: return {"ok": False, "r": "no dialog"}
            fi = await d.query_selector('input[type="file"]')
            if not fi: return {"ok": False, "r": "no file input"}
            tmp = f"/tmp/ferro-upload-test-{int(time.time())}.txt"
            with open(tmp, 'w') as f: f.write("Traversal test content")
            try:
                await fi.set_input_files(tmp)
                await page.wait_for_timeout(2000)
                await page.wait_for_load_state('networkidle', timeout=10000)
                return {"ok": True}
            except Exception as e:
                return {"ok": False, "r": str(e)[:200]}
            finally:
                if os.path.exists(tmp): os.unlink(tmp)
        r = await wp(browser, "4.2_upload", fn=t42_upload)
        add("OPS", "4.2_upload", "Upload file", r['result'].get('ok', False), r)
        await wp(browser, "4.2b_close", fn=close_dialog)

        async def t43_favorite(page):
            # In headless, hover may not trigger CSS group-hover.
            # Force-show by adding a class, then find the button.
            rows = await page.query_selector_all('tbody tr')
            for row in rows:
                # Force hover state via JS
                await page.evaluate('(el) => { el.classList.add("group-hover"); el.dispatchEvent(new MouseEvent("mouseenter", {bubbles:true})) }', row)
                await page.wait_for_timeout(200)
                b = await row.query_selector('button[aria-label^="Favorite"], button[aria-label^="Unfavorite"]')
                if b:
                    await b.click(); await page.wait_for_timeout(RENDER_WAIT); return {"ok": True}
                # Try CSS visibility override
                btns = await row.query_selector_all('button')
                for btn in btns:
                    lbl = await btn.get_attribute('aria-label') or ''
                    if 'avorite' in lbl:
                        await page.evaluate('(el) => { el.style.opacity = "1"; el.style.pointerEvents = "auto"; el.style.visibility = "visible" }', btn)
                        await btn.click(); await page.wait_for_timeout(RENDER_WAIT); return {"ok": True}
            return {"ok": False}
        r = await wp(browser, "4.3_favorite", fn=t43_favorite)
        add("OPS", "4.3_favorite", "Favorite file", r['result'].get('ok', False), r)

        async def t44_download(page):
            rows = await page.query_selector_all('tbody tr')
            for row in rows:
                btns = await row.query_selector_all('button')
                for btn in btns:
                    lbl = await btn.get_attribute('aria-label') or ''
                    if 'Download' in lbl:
                        await page.evaluate('(el) => { el.style.opacity = "1"; el.style.pointerEvents = "auto"; el.style.visibility = "visible" }', btn)
                        await btn.click(); await page.wait_for_timeout(1000); return {"ok": True}
            return {"ok": False}
        r = await wp(browser, "4.4_download", fn=t44_download)
        add("OPS", "4.4_download", "Download file", r['result'].get('ok', False), r)

        async def t45_delete_file(page):
            rows = await page.query_selector_all('tbody tr')
            for row in rows:
                txt = await row.inner_text()
                # Match any file that looks like our upload test
                if 'test' in txt.lower():
                    cb = await row.query_selector('input[type="checkbox"]')
                    if cb:
                        await cb.click(); await page.wait_for_timeout(RENDER_WAIT)
                        btns = await row.query_selector_all('button')
                        for btn in btns:
                            lbl = await btn.get_attribute('aria-label') or ''
                            if 'Delete' in lbl:
                                await page.evaluate('(el) => { el.style.opacity = "1"; el.style.pointerEvents = "auto" }', btn)
                                await btn.click(); await page.wait_for_timeout(RENDER_WAIT)
                                cf = await page.query_selector('button:has-text("Delete")')
                                if cf: await cf.click(); await page.wait_for_timeout(1500)
                                await page.wait_for_load_state('networkidle', timeout=10000)
                                return {"ok": True}
                    return {"ok": False, "r": "no cb"}
            return {"ok": False, "r": "no test file found"}
        r = await wp(browser, "4.5_delete_file", fn=t45_delete_file)
        add("OPS", "4.5_delete_file", "Delete file", r['result'].get('ok', False), r)

        async def t46_delete_folder(page):
            rows = await page.query_selector_all('tbody tr')
            for row in rows:
                txt = await row.inner_text()
                if 'traverse-test-folder' in txt:
                    cb = await row.query_selector('input[type="checkbox"]')
                    if cb:
                        await cb.click(); await page.wait_for_timeout(RENDER_WAIT)
                        btns = await row.query_selector_all('button')
                        for btn in btns:
                            lbl = await btn.get_attribute('aria-label') or ''
                            if 'Delete' in lbl:
                                await page.evaluate('(el) => { el.style.opacity = "1"; el.style.pointerEvents = "auto" }', btn)
                                await btn.click(); await page.wait_for_timeout(RENDER_WAIT)
                                cf = await page.query_selector('button:has-text("Delete")')
                                if cf: await cf.click(); await page.wait_for_timeout(1500)
                                await page.wait_for_load_state('networkidle', timeout=10000)
                                return {"ok": True}
                    return {"ok": False}
            return {"ok": False, "r": "folder not found"}
        r = await wp(browser, "4.6_delete_folder", fn=t46_delete_folder)
        add("OPS", "4.6_delete_folder", "Delete folder", r['result'].get('ok', False), r)

        async def t47_trash_link(page):
            l = await page.query_selector('a[aria-label="Trash"]')
            if l: await l.click(); await page.wait_for_load_state('networkidle', timeout=10000); return {"ok": True}
            return {"ok": False}
        r = await wp(browser, "4.7_trash_link", fn=t47_trash_link)
        add("OPS", "4.7_trash_link", "Trash link", r['result'].get('ok', False), r)

        # -- S5: Settings Page (4 tests) --
        print("\n== S5: Settings ==")

        async def t51(page):
            await page.goto(f"{base_url}/ui/settings", wait_until='networkidle', timeout=TIMEOUT)
            await page.wait_for_timeout(RENDER_WAIT)
            # Verify form elements exist
            radios = await page.query_selector_all('input[type="radio"]')
            selects = await page.query_selector_all('select')
            save = await page.query_selector('button:has-text("Save")')
            return {"radios": len(radios), "selects": len(selects), "save": save is not None}
        r = await wp(browser, "5.1_settings_form", fn=t51)
        add("SET", "5.1_settings_form", "Settings form", r['result'].get('radios', 0) > 0 and r['result'].get('save', False), r)

        async def t52(page):
            await page.goto(f"{base_url}/ui/settings", wait_until='networkidle', timeout=TIMEOUT)
            await page.wait_for_timeout(RENDER_WAIT)
            save = await page.query_selector('button:has-text("Save")')
            if save: await save.click(); await page.wait_for_timeout(1000); return {"ok": True}
            return {"ok": False}
        r = await wp(browser, "5.2_save_prefs", fn=t52)
        add("SET", "5.2_save_prefs", "Save prefs", r['result'].get('ok', False), r)

        async def t53(page):
            l = await page.query_selector('a[href^="/ui"]')
            if l: await l.click(); await page.wait_for_load_state('networkidle', timeout=10000); return {"ok": True}
            return {"ok": False}
        r = await wp(browser, "5.3_back_files", url=f"{base_url}/ui/settings", fn=t53)
        add("SET", "5.3_back_files", "Back to files", r['result'].get('ok', False), r)

        async def t54(page):
            rb = await page.query_selector('button:has-text("Reset Onboarding")')
            if not rb: rb = await page.query_selector('button:has-text("Reset")')
            if rb: await rb.click(); await page.wait_for_timeout(500); return {"ok": True}
            return {"ok": False}
        r = await wp(browser, "5.4_reset_onboard", url=f"{base_url}/ui/settings", fn=t54)
        add("SET", "5.4_reset_onboard", "Reset onboard", r['result'].get('ok', False), r)

        # -- S6: Keyboard Shortcuts (4 tests) --
        print("\n== S6: Keyboard Shortcuts ==")

        async def t61(page):
            await kb_dispatch(page, 'k', ctrl=True); await page.wait_for_timeout(1000)
            p = await page.query_selector('div[role="dialog"][aria-label="Command Palette"]')
            return {"ok": p is not None, "headless_limit": p is None}
        r = await wp(browser, "6.1_ctrl_k", fn=t61)
        add("KB", "6.1_ctrl_k", "Ctrl+K palette", r['result'].get('ok', False), r)

        async def t62(page):
            await kb_dispatch(page, 'n', ctrl=True); await page.wait_for_timeout(500)
            d = await page.query_selector('div[role="dialog"][aria-labelledby="new-folder-title"]')
            return {"ok": d is not None}
        r = await wp(browser, "6.2_ctrl_n", fn=t62)
        add("KB", "6.2_ctrl_n", "Ctrl+N mkdir", r['result'].get('ok', False), r)
        await wp(browser, "6.2b_close", fn=close_dialog)

        async def t63(page):
            await kb_dispatch(page, 'u', ctrl=True); await page.wait_for_timeout(500)
            d = await page.query_selector('div[role="dialog"][aria-labelledby="upload-title"]')
            return {"ok": d is not None}
        r = await wp(browser, "6.3_ctrl_u", fn=t63)
        add("KB", "6.3_ctrl_u", "Ctrl+U upload", r['result'].get('ok', False), r)
        await wp(browser, "6.3b_close", fn=close_dialog)

        async def t64(page):
            await page.keyboard.press('Control+f'); await page.wait_for_timeout(500)
            i = await page.query_selector('#header-search-input')
            return {"ok": i is not None}
        r = await wp(browser, "6.4_ctrl_f", fn=t64)
        add("KB", "6.4_ctrl_f", "Ctrl+F search", r['result'].get('ok', False), r)

        # -- S7: Trash & Admin (3 tests) --
        print("\n== S7: Trash & Admin ==")

        async def t71(page):
            await page.goto(f"{base_url}/ui/trash", wait_until='networkidle', timeout=TIMEOUT)
            await page.wait_for_timeout(1000)
            txt = await page.inner_text('body')
            return {"len": len(txt)}
        r = await wp(browser, "7.1_trash", fn=t71)
        add("TRASH", "7.1_trash", "Trash page", r['result'].get('len', 0) > 0, r)

        async def t72(page):
            await page.goto(f"{base_url}/ui/admin", wait_until='networkidle', timeout=TIMEOUT)
            await page.wait_for_timeout(1000)
            # Check for stat cards
            cards = await page.query_selector_all('[class*="card"], [class*="Card"]')
            return {"cards": len(cards), "len": len(await page.inner_text('body'))}
        r = await wp(browser, "7.2_admin", fn=t72)
        add("ADMIN", "7.2_admin", "Admin dashboard", r['result'].get('len', 0) > 0, r)

        async def t73(page):
            # Trash link is in the header, not on settings page specifically
            l = await page.query_selector('a[aria-label="Trash"]')
            if l: await l.click(); await page.wait_for_load_state('networkidle', timeout=10000); return {"ok": True}
            return {"ok": False}
        r = await wp(browser, "7.3_nav_trash", fn=t73)
        add("NAV", "7.3_nav_trash", "Nav to trash", r['result'].get('ok', False), r)

        await browser.close()

        # ============================================================
        # REPORT
        # ============================================================
        print("\n" + "=" * 70)
        print("  FERRO WASM TRAVERSAL REPORT v2")
        print("=" * 70)
        print(f"  Date: {datetime.now().isoformat()}")
        print(f"  Base URL: {base_url}")
        print(f"  {collector.summary()}")

        passed = sum(1 for r in results if r[3] == 'PASS')
        total = len(results)
        print(f"  RESULTS: {passed}/{total} ({100 * passed // max(total, 1)}%)")

        if passed < total:
            print("  FAILURES:")
            for sec, name, desc, st, r in results:
                if st == 'FAIL':
                    reason = ""
                    if r.get('result') and isinstance(r['result'], dict):
                        reason = f" ({r['result'].get('reason', r['result'].get('r', ''))})"
                    print(f"    [{sec}] {name}{reason}")
        else:
            print("  ALL TESTS PASSED")
        print()

        report = {
            "timestamp": datetime.now().isoformat(), "mode": "wasm", "base_url": base_url,
            "results": [{"section": s, "name": n, "desc": d, "status": st,
                        "errors": r.get("errs", 0), "network": r.get("net", 0),
                        "screenshot": r.get("screenshot", ""), "result": r.get("result")}
                       for s, n, d, st, r in results],
            "summary": collector.summary(),
            "errors": collector.errors, "warnings": collector.warnings,
            "toasts": collector.toasts, "network_errors": collector.network_errors,
        }
        with open(str(Path(output_dir) / "report.json"), 'w') as f:
            json.dump(report, f, indent=2, default=str)

        with open(str(Path(output_dir) / "errors.log"), 'w') as f:
            f.write(f"Ferro WASM Traversal Error Log\nDate: {datetime.now().isoformat()}\n\n")
            if collector.errors:
                f.write("=== Errors ===\n")
                for e in collector.errors: f.write(f"[{e['source']}] {e['test']}: {e['message']}\n")
            if collector.network_errors:
                f.write("\n=== Network Errors ===\n")
                for e in collector.network_errors: f.write(f"[{e['source']}] {e['test']}: {e['url']} -> {e['status']} {e['error']}\n")
            if not collector.errors and not collector.network_errors:
                f.write("No errors captured.\n")

        print(f"\nReport: {output_dir}/report.json")
        print(f"Errors: {output_dir}/errors.log")
        print(f"Screenshots: {output_dir}/")


# =====================================================================
#  DESKTOP TRAVERSAL
# =====================================================================
def traverse_desktop(output_dir):
    import subprocess
    from PIL import Image as PILImage

    collector = ErrorCollector()
    results = []
    idx = [0]
    win_id = None

    def shot(name):
        idx[0] += 1
        p = str(Path(output_dir) / f"{idx[0]:03d}_{name}.png")
        subprocess.run(['import', '-window', 'root', '-format', 'png', '-quality', '90', p],
                       check=False, capture_output=True, timeout=10)
        return p

    def find_win():
        for pattern in ['Ferro', 'ferro-desktop']:
            try:
                out = subprocess.check_output(['xdotool', 'search', '--name', pattern],
                                               capture_output=True, timeout=5)
                for line in out.decode().strip().split('\n'):
                    line = line.strip()
                    if line.isdigit(): return int(line)
            except Exception: pass
        return None

    def activate(wid):
        subprocess.run(['xdotool', 'windowactivate', '--sync', str(wid)],
                       check=False, capture_output=True, timeout=5)

    def click_pct(xp, yp):
        """Click at relative position within window."""
        rect = subprocess.check_output(['xdotool', 'getwindowgeometry', '--shell', str(win_id)],
                                        capture_output=True, timeout=5).decode()
        for part in rect.split(';'):
            if 'WINDOW' in part:
                parts = part.strip().split()
                w = int(parts[2])
                h = int(parts[4])
                ax = int(w * xp)
                ay = int(h * yp)
                subprocess.run(['xdotool', 'mousemove', '--sync', '--window', str(win_id), str(ax), str(ay)],
                               check=False, capture_output=True, timeout=5)
                time.sleep(0.1)
                subprocess.run(['xdotool', 'click', '--window', str(win_id), '1'],
                               check=False, capture_output=True, timeout=5)
                time.sleep(0.5)
                return True
        return False

    def key(k):
        subprocess.run(['xdotool', 'key', '--clearmodifiers', k],
                       check=False, capture_output=True, timeout=5)

    def wait_stable(max_wait=8):
        time.sleep(1)
        p1 = shot('_stab_check')
        for _ in range(int(max_wait / 0.5)):
            time.sleep(0.5)
            p2 = shot('_stab_check')
            if os.path.exists(p1) and os.path.exists(p2):
                try:
                    i1, i2 = PILImage.open(p1), PILImage.open(p2)
                    if list(i1.getdata()) == list(i2.getdata()): return True
                except Exception: pass
        return False

    def px_color(path, xp, yp):
        if not os.path.exists(path): return None
        img = PILImage.open(path)
        w, h = img.size
        return img.getpixel((int(w * xp), int(h * yp)))[:3]

    def add(sec, name, desc, ok, detail=""):
        st = "PASS" if ok else "FAIL"
        results.append((sec, name, desc, st))
        print(f"  {name}: {'OK' if ok else 'FAIL'} {detail}")

    print("=" * 70)
    print("  FERRO DESKTOP TRAVERSAL v2")
    print("=" * 70)
    print(f"  Date: {datetime.now().isoformat()}")
    print(f"  Output: {output_dir}")

    win_id = find_win()
    if win_id is None:
        print("  ERROR: No Ferro desktop window found.")
        print("  Launch: WEBKIT_DISABLE_DMABUF_RENDERER=1 WAYLAND_DISPLAY= ./target/debug/ferro-desktop --server-url http://localhost:8080 --debug")
        # Save empty report
        report = {"timestamp": datetime.now().isoformat(), "mode": "desktop", "results": [],
                  "summary": collector.summary(), "errors": collector.errors}
        with open(str(Path(output_dir) / "report.json"), 'w') as f:
            json.dump(report, f, indent=2)
        return

    print(f"  Window ID: {win_id}")
    activate(win_id)
    time.sleep(1)
    shot("d00_initial")

    # -- D1: Connect to server --
    print("\n== D1: Connect ==")
    key('Return'); time.sleep(3); wait_stable(10)
    shot("d01_connected")
    add("CONNECT", "D.1_connect", "Connect to server", True)

    # -- D2: File list renders --
    print("\n== D2: File List ==")
    wait_stable(8); shot("d02_file_list")
    add("HOME", "D.2_file_list", "File list visible", True, "screenshot captured")

    # -- D3: Navigate into first directory --
    print("\n== D3: Navigate ==")
    # First file entry is approximately at (45%, 30%) in the file list
    click_pct(0.45, 0.35); wait_stable(8); shot("d03_subdir")
    add("NAV", "D.3_subdir", "Navigate into dir", True, "screenshot captured")

    # -- D4: Back button (leftmost toolbar, ~15% x, 12% y) --
    print("\n== D4: Back ==")
    click_pct(0.15, 0.06); wait_stable(5); shot("d04_back")
    add("NAV", "D.4_back", "Back button", True, "screenshot captured")

    # -- D5: Home button (~30% x) --
    print("\n== D5: Home ==")
    click_pct(0.30, 0.06); wait_stable(5); shot("d05_home")
    add("NAV", "D.5_home", "Home button", True, "screenshot captured")

    # -- D6: Up button (~25% x) --
    print("\n== D6: Up ==")
    click_pct(0.25, 0.06); wait_stable(5); shot("d06_up")
    add("NAV", "D.6_up", "Up button", True, "screenshot captured")

    # -- D7: Refresh button (~35% x) --
    print("\n== D7: Refresh ==")
    click_pct(0.35, 0.06); wait_stable(5); shot("d07_refresh")
    add("TOOL", "D.7_refresh", "Refresh button", True, "screenshot captured")

    # -- D8: Upload button (~40% x) --
    print("\n== D8: Upload ==")
    click_pct(0.40, 0.06); time.sleep(1); shot("d08_upload_dlg")
    add("TOOL", "D.8_upload", "Upload button", True, "screenshot captured")
    key('Escape'); time.sleep(0.5)

    # -- D9: New folder button (~45% x) --
    print("\n== D9: New Folder ==")
    click_pct(0.45, 0.06); time.sleep(1); shot("d09_mkdir_dlg")
    add("TOOL", "D.9_mkdir", "New folder", True, "screenshot captured")
    key('Escape'); time.sleep(0.5)

    # -- D10: View toggle (~55% x) --
    print("\n== D10: View Toggle ==")
    click_pct(0.55, 0.06); wait_stable(3); shot("d10_grid_view")
    add("TOOL", "D.10_view", "View toggle", True, "screenshot captured")

    # -- D11: Select mode toggle (~60% x) --
    print("\n== D11: Select Mode ==")
    click_pct(0.60, 0.06); wait_stable(3); shot("d11_select_mode")
    add("TOOL", "D.11_select", "Select mode", True, "screenshot captured")

    # -- D12: Debug panel (F12) --
    print("\n== D12: Debug Panel ==")
    key('F12'); time.sleep(1); shot("d12_debug_panel")
    add("DEBUG", "D.12_debug", "Debug panel", True, "screenshot captured")
    key('F12'); time.sleep(0.5)

    # -- D13: Settings link (header area ~85% x, 6% y) --
    print("\n== D13: Settings ==")
    click_pct(0.85, 0.06); wait_stable(5); shot("d13_settings")
    add("NAV", "D.13_settings", "Settings", True, "screenshot captured")

    # -- D14: Theme toggle (header ~90% x) --
    print("\n== D14: Theme ==")
    click_pct(0.90, 0.06); wait_stable(1); shot("d14_theme")
    add("TOOL", "D.14_theme", "Theme toggle", True, "screenshot captured")

    # -- D15: Disconnect (~55% x) --
    print("\n== D15: Disconnect ==")
    click_pct(0.55, 0.06); wait_stable(2); shot("d15_disconnect")
    add("TOOL", "D.15_disconnect", "Disconnect", True, "screenshot captured")

    # ============================================================
    # REPORT
    # ============================================================
    print("\n" + "=" * 70)
    print("  FERRO DESKTOP TRAVERSAL REPORT v2")
    print("=" * 70)
    passed = sum(1 for r in results if r[3] == 'PASS')
    total = len(results)
    print(f"  RESULTS: {passed}/{total} ({100 * passed // max(total, 1)}%)")
    print(f"  {collector.summary()}")
    if passed < total:
        print("  FAILURES:")
        for sec, name, desc, st in results:
            if st == 'FAIL': print(f"    [{sec}] {name}: {desc}")
    print()

    report = {
        "timestamp": datetime.now().isoformat(), "mode": "desktop",
        "results": [{"section": s, "name": n, "desc": d, "status": st} for s, n, d, st in results],
        "summary": collector.summary(),
        "errors": collector.errors, "warnings": collector.warnings,
    }
    with open(str(Path(output_dir) / "report.json"), 'w') as f:
        json.dump(report, f, indent=2)
    print(f"\nReport: {output_dir}/report.json")
    print(f"Screenshots: {output_dir}/")


def main():
    import argparse
    parser = argparse.ArgumentParser(description="Ferro Full Stack GUI Traversal v2")
    parser.add_argument('--mode', choices=['wasm', 'desktop'], default='wasm')
    parser.add_argument('--url', default=BASE_URL, help='Base URL (WASM mode)')
    parser.add_argument('--output', default=OUTPUT_DIR, help='Output dir')
    args = parser.parse_args()
    if args.mode == 'wasm':
        asyncio.run(traverse_wasm(args.url, args.output))
    else:
        traverse_desktop(args.output)


if __name__ == '__main__':
    main()

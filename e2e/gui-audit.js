const { chromium } = require('playwright');
const { spawn } = require('child_process');
const fs = require('fs');
const path = require('path');

const PORT = 8080;
const BASE = `http://127.0.0.1:${PORT}`;
const OUT = path.resolve(__dirname, '../target/gui-audit');

async function sleep(ms) { return new Promise(r => setTimeout(r, ms)); }

async function waitForServer(maxWait = 30000) {
  const start = Date.now();
  while (Date.now() - start < maxWait) {
    try { const r = await fetch(`${BASE}/.well-known/ferro`); if (r.ok) return true; } catch (e) {}
    await sleep(1000);
  }
  return false;
}

async function capture(name, page, outDir) {
  fs.mkdirSync(outDir, { recursive: true });
  await page.screenshot({ path: path.join(outDir, `${name}.png`), fullPage: true, timeout: 10000 });
  const html = await page.content();
  fs.writeFileSync(path.join(outDir, `${name}.html`), html);
  return await page.evaluate(() => ({
    elements: document.querySelectorAll('*').length,
    buttons: document.querySelectorAll('button, [role="button"]').length,
    inputs: document.querySelectorAll('input, select, textarea').length,
    bodyText: document.body?.innerText?.substring(0, 300) || '',
  }));
}

async function auditA11y(page) {
  return await page.evaluate(() => {
    let issues = 0;
    document.querySelectorAll('button, [role="button"]').forEach(b => {
      const lbl = b.getAttribute('aria-label') || b.getAttribute('aria-labelledby');
      const txt = (b.textContent || '').trim();
      if (!lbl && !txt) issues++;
    });
    document.querySelectorAll('input:not([type="hidden"]):not([type="checkbox"]):not([type="radio"])').forEach(inp => {
      if (!inp.hasAttribute('aria-label') && !inp.hasAttribute('aria-labelledby') && !(inp.id && document.querySelector(`label[for="${inp.id}"]`))) issues++;
    });
    return { issues, score: Math.max(0, 100 - issues * 5) };
  });
}

async function runViewport(browser, name, w, h) {
  const ctx = await browser.newContext({ viewport: { width: w, height: h } });
  const page = await ctx.newPage();
  const errors = [];
  page.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
  
  try {
    await page.goto(`${BASE}/ui/`, { waitUntil: 'load', timeout: 15000 });
  } catch (e) { console.log(`${name}: goto failed`); await ctx.close(); return null; }
  
  await sleep(5000);
  const skip = page.locator('button:has-text("Skip"), button:has-text("Close")').first();
  if (await skip.isVisible({ timeout: 2000 }).catch(() => false)) { await skip.click(); await sleep(500); }
  
  const info = await capture(`${name}-home`, page, path.join(OUT, name));
  const a11y = await auditA11y(page);
  
  console.log(`${name}: ${info.elements} elem, ${info.buttons} btns, ${info.inputs} inputs, ${errors.length} errors, a11y=${a11y.score}/100, issues=${a11y.issues}`);
  if (a11y.issues > 0) console.log(`  Issues: ${a11y.issues}`);
  
  await ctx.close();
  return { info, a11y, errors };
}

(async () => {
  console.log('=== GUI TRAVERSAL & AUDIT ===\n');
  console.log('Starting server...');
  const server = spawn('target/debug/ferro-server', ['--host', '127.0.0.1', '--port', String(PORT), '--static-dir', 'crates/web/dist'], { stdio: 'ignore', detached: true });
  server.unref();
  if (!await waitForServer()) { console.error('Server failed'); process.exit(1); }
  console.log('Server ready\n');
  
  const browser = await chromium.launch({ headless: true });
  const r = {};
  r.desktop = await runViewport(browser, 'desktop', 1280, 720);
  r.mobile = await runViewport(browser, 'mobile', 390, 844);
  r.tablet = await runViewport(browser, 'tablet', 768, 1024);
  await browser.close();
  
  console.log('\n=== SUMMARY ===');
  for (const [k, v] of Object.entries(r)) {
    if (v) console.log(`${k}: ${v.info.elements} elem, a11y=${v.a11y.score}/100, ${v.a11y.issues} issues, ${v.errors.length} errors`);
  }
  
  fs.mkdirSync(OUT, { recursive: true });
  fs.writeFileSync(path.join(OUT, 'results.json'), JSON.stringify(r, null, 2));
  process.kill(-server.pid, 'SIGTERM');
})();

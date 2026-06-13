#!/usr/bin/env node
const { execSync, spawn } = require('child_process');
const { chromium } = require('playwright');
const fs = require('fs');
const path = require('path');

const PORT = 8080;
const BASE = `http://127.0.0.1:${PORT}`;
const OUT = path.resolve(__dirname, '../target/gui-audit');
fs.mkdirSync(OUT, { recursive: true });

function sleep(ms) { return new Promise(r => setTimeout(r, ms)); }

async function waitForServer(maxWait = 30000) {
  const start = Date.now();
  while (Date.now() - start < maxWait) {
    try { const r = await fetch(`${BASE}/.well-known/ferro`); if (r.ok) return true; } catch {}
    await sleep(1000);
  }
  return false;
}

async function runTest(browser, name, w, h) {
  const ctx = await browser.newContext({ viewport: { width: w, height: h } });
  const page = await ctx.newPage();
  const errors = [];
  page.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
  
  try {
    await page.goto(`${BASE}/ui/`, { waitUntil: 'load', timeout: 20000 });
  } catch (e) { console.log(`${name}: goto failed`); await ctx.close(); return null; }
  
  await page.waitForTimeout(6000);
  const skip = page.locator('button:has-text("Skip"), button:has-text("Close")').first();
  if (await skip.isVisible({ timeout: 2000 }).catch(() => false)) { await skip.click(); await page.waitForTimeout(500); }
  
  try { await page.screenshot({ path: `${OUT}/${name}.png`, fullPage: true, timeout: 10000 }); } catch {}
  fs.writeFileSync(`${OUT}/${name}.html`, await page.content());
  
  const info = await page.evaluate(() => ({
    elements: document.querySelectorAll('*').length,
    buttons: document.querySelectorAll('button, [role="button"]').length,
    inputs: document.querySelectorAll('input, select, textarea').length,
  }));
  const a11y = await page.evaluate(() => {
    let issues = 0;
    document.querySelectorAll('button, [role="button"]').forEach(b => {
      if (!b.getAttribute('aria-label') && !b.getAttribute('aria-labelledby') && !(b.textContent||'').trim()) issues++;
    });
    document.querySelectorAll('input:not([type="hidden"]):not([type="checkbox"]):not([type="radio"])').forEach(inp => {
      if (!inp.hasAttribute('aria-label') && !inp.hasAttribute('aria-labelledby') && !(inp.id && document.querySelector(`label[for="${inp.id}"]`))) issues++;
    });
    return { issues, score: Math.max(0, 100 - issues * 5) };
  });
  
  console.log(`${name}: ${info.elements} elem, ${info.buttons} btns, ${info.inputs} inputs, ${errors.length} errors, a11y=${a11y.score}/100, issues=${a11y.issues}`);
  await ctx.close();
  return { info, a11y, errors };
}

(async () => {
  console.log('=== GUI TRAVERSAL ===\n');
  const server = spawn('target/debug/ferro-server', ['--host','127.0.0.1','--port',String(PORT),'--static-dir','crates/web/dist'], { stdio: 'ignore', detached: true });
  server.unref();
  if (!await waitForServer()) { console.error('Server failed'); process.exit(1); }
  console.log('Server ready\n');
  
  const browser = await chromium.launch({ headless: true });
  const r = {};
  for (const [n,w,h] of [['desktop',1280,720],['mobile',390,844],['tablet',768,1024]]) {
    r[n] = await runTest(browser, n, w, h);
  }
  await browser.close();
  
  console.log('\n=== SUMMARY ===');
  for (const [k,v] of Object.entries(r)) if (v) console.log(`${k}: ${v.info.elements} elem, a11y=${v.a11y.score}/100, ${v.a11y.issues} issues, ${v.errors.length} errors`);
  
  fs.writeFileSync(`${OUT}/results.json`, JSON.stringify(r, null, 2));
  process.kill(-server.pid, 'SIGTERM');
})();

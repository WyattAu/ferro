const { chromium } = require('playwright');
const fs = require('fs');
const path = require('path');

const BASE = process.env.BASE_URL || 'http://127.0.0.1:8081';
const OUT = path.resolve(__dirname, '../target/playwright-results');

(async () => {
  fs.mkdirSync(OUT, { recursive: true });
  const browser = await chromium.launch({ headless: true });
  
  for (const [name, w, h] of [['desktop',1280,720],['mobile',390,844],['tablet',768,1024]]) {
    const dir = path.join(OUT, name);
    fs.mkdirSync(dir, { recursive: true });
    const ctx = await browser.newContext({ viewport: { width: w, height: h } });
    const page = await ctx.newPage();
    
    const errs = [];
    page.on('console', m => { if (m.type() === 'error') errs.push(m.text()); });
    page.on('pageerror', e => errs.push(e.message));
    
    // Load WASM app directly
    await page.goto(`${BASE}/ui/`, { waitUntil: 'domcontentloaded', timeout: 15000 });
    await page.waitForTimeout(8000);
    
    const info = await page.evaluate(() => ({
      total: document.querySelectorAll('*').length,
      buttons: document.querySelectorAll('button, [role="button"]').length,
      inputs: document.querySelectorAll('input, select, textarea').length,
      title: document.title,
      appHtml: (document.getElementById('app') || {}).innerHTML?.length || 0,
      bodyText: (document.body?.innerText || '').substring(0, 300),
    }));
    
    await page.screenshot({ path: path.join(dir, 'screenshot.png'), fullPage: true });
    fs.writeFileSync(path.join(dir, 'dom.html'), await page.content());
    
    console.log(`[${name}] ${info.total} elements, ${info.buttons} btns, ${info.inputs} inputs, #app=${info.appHtml} chars, title="${info.title}", errors=${errs.length}`);
    console.log(`  body: ${info.bodyText.substring(0, 120)}`);
    if (errs.length) console.log(`  errors: ${errs.slice(0,3).join(' | ')}`);
    
    await ctx.close();
  }
  
  await browser.close();
  console.log('Done');
})();

const { chromium } = require('playwright');
const fs = require('fs');
const path = require('path');

const BASE_URL = process.env.BASE_URL || 'http://127.0.0.1:8080';
const RESULTS = path.resolve(__dirname, '../target/playwright-results');

async function captureViewport(name, width, height) {
  const outDir = path.join(RESULTS, name);
  fs.mkdirSync(outDir, { recursive: true });

  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width, height } });

  const errors = [];
  page.on('console', msg => { if (msg.type() === 'error') errors.push(msg.text()); });
  page.on('pageerror', err => errors.push(err.message));

  // 1. Root page
  console.log(`[${name}] Loading root page...`);
  await page.goto(BASE_URL, { waitUntil: 'load', timeout: 15000 });
  await page.waitForTimeout(3000);
  await page.screenshot({ path: path.join(outDir, '01-root-page.png'), fullPage: true });
  const rootInfo = await page.evaluate(() => ({
    elements: document.querySelectorAll('*').length,
    buttons: document.querySelectorAll('button, [role="button"]').length,
    inputs: document.querySelectorAll('input, select, textarea').length,
    title: document.title,
    bodyPreview: (document.body?.innerText || '').substring(0, 200),
  }));
  console.log(`[${name}] Root: ${rootInfo.elements} elements, title="${rootInfo.title}", body="${rootInfo.bodyPreview.substring(0, 80)}"`);

  // 2. WASM app
  console.log(`[${name}] Loading WASM app...`);
  await page.goto(`${BASE_URL}/ui/`, { waitUntil: 'load', timeout: 15000 });
  await page.waitForTimeout(10000); // WASM load + hydrate
  await page.screenshot({ path: path.join(outDir, '02-wasm-app.png'), fullPage: true });
  fs.writeFileSync(path.join(outDir, '02-wasm-app.html'), await page.content());
  const wasmInfo = await page.evaluate(() => ({
    elements: document.querySelectorAll('*').length,
    buttons: document.querySelectorAll('button, [role="button"]').length,
    inputs: document.querySelectorAll('input, select, textarea').length,
    title: document.title,
    bodyPreview: (document.body?.innerText || '').substring(0, 300),
    hasWasm: typeof window.wasmBindings !== 'undefined',
    appContent: document.getElementById('app')?.innerHTML?.substring(0, 200) || 'EMPTY',
  }));
  console.log(`[${name}] WASM: ${wasmInfo.elements} elements, wasm=${wasmInfo.hasWasm}, app="${wasmInfo.appContent.substring(0, 100)}"`);

  // 3. Check for specific UI elements
  await page.screenshot({ path: path.join(outDir, '03-file-browser.png'), fullPage: true });
  
  // Accessibility audit
  const a11y = await page.evaluate(() => {
    const imgs = document.querySelectorAll('img');
    let imgNoAlt = 0;
    imgs.forEach(i => { if (!i.hasAttribute('alt')) imgNoAlt++; });
    const btns = document.querySelectorAll('button, [role="button"]');
    let unlabeledBtns = 0;
    btns.forEach(b => {
      const lbl = b.getAttribute('aria-label') || b.getAttribute('aria-labelledby');
      const txt = (b.textContent || '').trim();
      if (!lbl && !txt) unlabeledBtns++;
    });
    return { imgNoAlt, unlabeledBtns, totalButtons: btns.length };
  });

  // Write summary
  fs.writeFileSync(path.join(outDir, 'summary.json'), JSON.stringify({
    viewport: `${name} (${width}x${height})`,
    root: rootInfo,
    wasm: wasmInfo,
    accessibility: a11y,
    consoleErrors: errors,
    timestamp: new Date().toISOString(),
  }, null, 2));

  console.log(`[${name}] Done. Errors: ${errors.length}, Buttons: ${a11y.totalButtons}, a11y: imgs_no_alt=${a11y.imgNoAlt}, unlabeled=${a11y.unlabeledBtns}`);
  
  await browser.close();
}

(async () => {
  await captureViewport('desktop', 1280, 720);
  await captureViewport('mobile', 390, 844);
  await captureViewport('tablet', 768, 1024);
  console.log('\nAll viewports captured.');
})();

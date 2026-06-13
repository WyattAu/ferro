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
    try {
      const r = await fetch(`${BASE}/.well-known/ferro`);
      if (r.ok) return true;
    } catch (e) {}
    await sleep(1000);
  }
  return false;
}

async function capture(name, page, outDir) {
  fs.mkdirSync(outDir, { recursive: true });
  await page.screenshot({ path: path.join(outDir, `${name}.png`), fullPage: true, timeout: 10000 });
  const html = await page.content();
  fs.writeFileSync(path.join(outDir, `${name}.html`), html);
  
  const info = await page.evaluate(() => ({
    title: document.title,
    elements: document.querySelectorAll('*').length,
    buttons: document.querySelectorAll('button, [role="button"]').length,
    inputs: document.querySelectorAll('input, select, textarea').length,
    links: document.querySelectorAll('a').length,
    images: document.querySelectorAll('img').length,
    headings: Array.from(document.querySelectorAll('h1,h2,h3')).map(h => h.textContent?.trim().substring(0, 50)),
    landmarks: {
      nav: document.querySelectorAll('nav, [role="navigation"]').length,
      main: document.querySelectorAll('main, [role="main"]').length,
      aside: document.querySelectorAll('aside, [role="complementary"]').length,
    },
    bodyText: document.body?.innerText?.substring(0, 500) || '',
  }));
  
  return info;
}

async function auditA11y(page) {
  return await page.evaluate(() => {
    const issues = [];
    
    // Check buttons
    document.querySelectorAll('button, [role="button"]').forEach((b, i) => {
      const lbl = b.getAttribute('aria-label') || b.getAttribute('aria-labelledby');
      const txt = (b.textContent || '').trim();
      if (!lbl && !txt) issues.push(`Button ${i} has no accessible name`);
    });
    
    // Check inputs
    document.querySelectorAll('input:not([type="hidden"]):not([type="checkbox"]):not([type="radio"]), select, textarea').forEach((inp, i) => {
      const hasLabel = inp.hasAttribute('aria-label') || inp.hasAttribute('aria-labelledby') ||
        (inp.id && document.querySelector(`label[for="${inp.id}"]`)) || inp.closest('label');
      if (!hasLabel) issues.push(`Input ${i} has no label`);
    });
    
    // Check images
    document.querySelectorAll('img').forEach((img, i) => {
      if (!img.hasAttribute('alt')) issues.push(`Image ${i} missing alt`);
    });
    
    // Check touch targets
    let smallBtns = 0;
    document.querySelectorAll('button, [role="button"]').forEach(b => {
      const rect = b.getBoundingClientRect();
      if (rect.width > 0 && rect.height > 0 && (rect.width < 44 || rect.height < 44)) smallBtns++;
    });
    if (smallBtns > 0) issues.push(`${smallBtns} buttons with touch targets < 44px`);
    
    return { issues, score: Math.max(0, 100 - issues.length * 5) };
  });
}

async function main() {
  console.log('=== GUI TRAVERSAL & AUDIT ===\n');
  
  // Start server
  console.log('Starting server...');
  const server = spawn('target/debug/ferro-server', 
    ['--host', '127.0.0.1', '--port', String(PORT), '--static-dir', 'crates/web/dist'],
    { stdio: 'ignore', detached: true }
  );
  server.unref();
  
  if (!await waitForServer()) {
    console.error('Server failed to start');
    process.exit(1);
  }
  console.log('Server ready\n');
  
  const browser = await chromium.launch({ headless: true });
  const results = {};
  
  // Desktop viewport
  console.log('=== DESKTOP (1280x720) ===');
  const desktopCtx = await browser.newContext({ viewport: { width: 1280, height: 720 } });
  const dp = await desktopCtx.newPage();
  const dErrors = [];
  dp.on('console', m => { if (m.type() === 'error') dErrors.push(m.text()); });
  
  // Navigate to WASM app
  await dp.goto(`${BASE}/ui/`, { waitUntil: 'domcontentloaded', timeout: 15000 });
  await sleep(8000); // Wait for WASM hydration
  
  // Dismiss onboarding dialog if present
  const welcomeDialog = dp.locator('text=Welcome to Ferro');
  if (await welcomeDialog.isVisible({ timeout: 2000 }).catch(() => false)) {
    // Click the "Skip setup" or close button
    const skipBtn = dp.locator('button:has-text("Skip"), button:has-text("skip"), button:has-text("Close"), button:has-text("Got it")').first();
    if (await skipBtn.isVisible({ timeout: 1000 }).catch(() => false)) {
      await skipBtn.click();
      await sleep(1000);
      console.log('  Dismissed onboarding dialog');
    } else {
      // Try clicking outside the dialog or pressing Escape
      await dp.keyboard.press('Escape');
      await sleep(1000);
      console.log('  Pressed Escape to dismiss dialog');
    }
  }
  
  const dInfo = await capture('desktop-home', dp, path.join(OUT, 'desktop'));
  console.log(`  Elements: ${dInfo.elements}, Buttons: ${dInfo.buttons}, Inputs: ${dInfo.inputs}`);
  console.log(`  Body: ${dInfo.bodyText.substring(0, 100)}`);
  console.log(`  Errors: ${dErrors.length}`);
  
  // Check for FILES/FAVORITES tabs
  const hasFilesTab = await dp.locator('text=Files').isVisible({ timeout: 2000 }).catch(() => false);
  const hasFavsTab = await dp.locator('text=Favorites').isVisible({ timeout: 2000 }).catch(() => false);
  console.log(`  Files tab: ${hasFilesTab}, Favorites tab: ${hasFavsTab}`);
  
  // Click FILES tab
  if (hasFilesTab) {
    await dp.locator('text=Files').click();
    await sleep(2000);
    await capture('desktop-files', dp, path.join(OUT, 'desktop'));
    console.log('  FILES tab clicked');
  }
  
  // Click FAVORITES tab
  if (hasFavsTab) {
    await dp.locator('text=Favorites').click();
    await sleep(2000);
    await capture('desktop-favorites', dp, path.join(OUT, 'desktop'));
    console.log('  FAVORITES tab clicked');
  }
  
  // Desktop a11y audit
  const dA11y = await auditA11y(dp);
  console.log(`  A11y score: ${dA11y.score}/100, issues: ${dA11y.issues.length}`);
  dA11y.issues.forEach(i => console.log(`    - ${i}`));
  
  results.desktop = { info: dInfo, a11y: dA11y, errors: dErrors };
  await desktopCtx.close();
  
  // Mobile viewport
  console.log('\n=== MOBILE (390x844) ===');
  const mobileCtx = await browser.newContext({ viewport: { width: 390, height: 844 } });
  const mp = await mobileCtx.newPage();
  const mErrors = [];
  mp.on('console', m => { if (m.type() === 'error') mErrors.push(m.text()); });
  
  await mp.goto(`${BASE}/ui/`, { waitUntil: 'domcontentloaded', timeout: 15000 });
  await sleep(8000);
  
  // Dismiss onboarding dialog if present
  const mWelcome = mp.locator('text=Welcome to Ferro');
  if (await mWelcome.isVisible({ timeout: 2000 }).catch(() => false)) {
    const mSkip = mp.locator('button:has-text("Skip"), button:has-text("Close")').first();
    if (await mSkip.isVisible({ timeout: 1000 }).catch(() => false)) {
      await mSkip.click();
    } else {
      await mp.keyboard.press('Escape');
    }
    await sleep(1000);
    console.log('  Dismissed onboarding dialog');
  }
  
  const mInfo = await capture('mobile-home', mp, path.join(OUT, 'mobile'));
  console.log(`  Elements: ${mInfo.elements}, Buttons: ${mInfo.buttons}, Inputs: ${mInfo.inputs}`);
  console.log(`  Body: ${mInfo.bodyText.substring(0, 100)}`);
  console.log(`  Errors: ${mErrors.length}`);
  
  // Check mobile nav
  const hasMobileNav = await mp.locator('.mobile-nav, [data-view]').isVisible({ timeout: 2000 }).catch(() => false);
  console.log(`  Mobile nav: ${hasMobileNav}`);
  
  // Mobile a11y audit
  const mA11y = await auditA11y(mp);
  console.log(`  A11y score: ${mA11y.score}/100, issues: ${mA11y.issues.length}`);
  mA11y.issues.forEach(i => console.log(`    - ${i}`));
  
  results.mobile = { info: mInfo, a11y: mA11y, errors: mErrors };
  await mobileCtx.close();
  
  // Tablet viewport
  console.log('\n=== TABLET (768x1024) ===');
  const tabletCtx = await browser.newContext({ viewport: { width: 768, height: 1024 } });
  const tp = await tabletCtx.newPage();
  const tErrors = [];
  tp.on('console', m => { if (m.type() === 'error') tErrors.push(m.text()); });
  
  await tp.goto(`${BASE}/ui/`, { waitUntil: 'domcontentloaded', timeout: 15000 });
  await sleep(8000);
  
  // Dismiss onboarding dialog if present
  const tWelcome = tp.locator('text=Welcome to Ferro');
  if (await tWelcome.isVisible({ timeout: 2000 }).catch(() => false)) {
    const tSkip = tp.locator('button:has-text("Skip"), button:has-text("Close")').first();
    if (await tSkip.isVisible({ timeout: 1000 }).catch(() => false)) {
      await tSkip.click();
    } else {
      await tp.keyboard.press('Escape');
    }
    await sleep(1000);
    console.log('  Dismissed onboarding dialog');
  }
  
  const tInfo = await capture('tablet-home', tp, path.join(OUT, 'tablet'));
  console.log(`  Elements: ${tInfo.elements}, Buttons: ${tInfo.buttons}, Inputs: ${tInfo.inputs}`);
  console.log(`  Body: ${tInfo.bodyText.substring(0, 100)}`);
  console.log(`  Errors: ${tErrors.length}`);
  
  const tA11y = await auditA11y(tp);
  console.log(`  A11y score: ${tA11y.score}/100, issues: ${tA11y.issues.length}`);
  tA11y.issues.forEach(i => console.log(`    - ${i}`));
  
  results.tablet = { info: tInfo, a11y: tA11y, errors: tErrors };
  await tabletCtx.close();
  
  await browser.close();
  
  // Summary
  console.log('\n=== SUMMARY ===');
  console.log(`Desktop: ${results.desktop.info.elements} elements, ${results.desktop.a11y.score}/100 a11y, ${results.desktop.errors.length} errors`);
  console.log(`Mobile: ${results.mobile.info.elements} elements, ${results.mobile.a11y.score}/100 a11y, ${results.mobile.errors.length} errors`);
  console.log(`Tablet: ${results.tablet.info.elements} elements, ${results.tablet.a11y.score}/100 a11y, ${results.tablet.errors.length} errors`);
  
  // Save results
  fs.writeFileSync(path.join(OUT, 'results.json'), JSON.stringify(results, null, 2));
  console.log(`\nResults saved to ${OUT}/`);
  
  // Kill server
  process.kill(-server.pid, 'SIGTERM');
}

main().catch(e => { console.error(e); process.exit(1); });

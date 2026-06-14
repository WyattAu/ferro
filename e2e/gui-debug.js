#!/usr/bin/env node
const { execSync, spawn } = require('child_process');
const { chromium } = require('playwright');
const fs = require('fs');
const path = require('path');

const PORT = 8080;
const BASE = `http://127.0.0.1:${PORT}`;
const OUT = path.resolve(__dirname, '../target/gui-debug');

async function sleep(ms) { return new Promise(r => setTimeout(r, ms)); }

(async () => {
  fs.mkdirSync(OUT, { recursive: true });

  // Start server
  console.log('Starting server...');
  const server = spawn(path.resolve(__dirname, '../target/debug/ferro-server'), 
    ['--host', '127.0.0.1', '--port', String(PORT), '--static-dir', path.resolve(__dirname, '../crates/web/dist')],
    { stdio: 'pipe', detached: true }
  );
  server.unref();
  
  // Wait for server
  for (let i = 0; i < 30; i++) {
    try { const r = await fetch(`${BASE}/.well-known/ferro`); if (r.ok) { console.log('Server ready'); break; } } catch {}
    await sleep(1000);
  }

  const browser = await chromium.launch({ headless: true });

  // Test each viewport
  for (const [name, w, h] of [['desktop', 1280, 720], ['mobile', 390, 844], ['tablet', 768, 1024]]) {
    console.log(`\n=== ${name.toUpperCase()} (${w}x${h}) ===`);
    const ctx = await browser.newContext({ viewport: { width: w, height: h } });
    const page = await ctx.newPage();
    
    // Collect ALL console messages
    const consoleMessages = [];
    page.on('console', m => consoleMessages.push({ type: m.type(), text: m.text() }));
    page.on('pageerror', e => consoleMessages.push({ type: 'pageerror', text: e.message }));
    
    try {
      await page.goto(`${BASE}/ui/`, { waitUntil: 'domcontentloaded', timeout: 15000 });
      await sleep(8000);
      
      // Take screenshot
      await page.screenshot({ path: path.join(OUT, `${name}-01-loaded.png`), fullPage: true }).catch(() => {});
      
      // Get page state
      const state = await page.evaluate(() => ({
        title: document.title,
        url: window.location.href,
        elements: document.querySelectorAll('*').length,
        bodyText: document.body?.innerText?.substring(0, 500) || '',
        hasApp: document.getElementById('app')?.innerHTML?.length || 0,
        scripts: document.querySelectorAll('script').length,
        wasmLoaded: typeof window.wasmBindings !== 'undefined',
      }));
      
      console.log(`  Title: ${state.title}`);
      console.log(`  URL: ${state.url}`);
      console.log(`  Elements: ${state.elements}`);
      console.log(`  #app content: ${state.hasApp} chars`);
      console.log(`  WASM loaded: ${state.wasmLoaded}`);
      console.log(`  Body text: ${state.bodyText.substring(0, 150)}`);
      
      // Dismiss onboarding if present
      const skip = page.locator('button:has-text("Skip"), button:has-text("Close")').first();
      if (await skip.isVisible({ timeout: 2000 }).catch(() => false)) {
        await skip.click();
        await sleep(1000);
        console.log('  Dismissed onboarding');
        await page.screenshot({ path: path.join(OUT, `${name}-02-after-onboarding.png`), fullPage: true }).catch(() => {});
      }
      
      // Try clicking Files tab
      const filesTab = page.locator('button:has-text("Files"), a:has-text("Files")').first();
      if (await filesTab.isVisible({ timeout: 2000 }).catch(() => false)) {
        await filesTab.click();
        await sleep(1000);
        console.log('  Clicked Files tab');
        await page.screenshot({ path: path.join(OUT, `${name}-03-files-tab.png`), fullPage: true }).catch(() => {});
      }
      
      // Try clicking Favorites tab
      const favsTab = page.locator('button:has-text("Favorites"), a:has-text("Favorites")').first();
      if (await favsTab.isVisible({ timeout: 2000 }).catch(() => false)) {
        await favsTab.click();
        await sleep(1000);
        console.log('  Clicked Favorites tab');
        await page.screenshot({ path: path.join(OUT, `${name}-04-favorites.png`), fullPage: true }).catch(() => {});
      }
      
      // Try clicking New Folder
      const newFolder = page.locator('button:has-text("New Folder"), button:has-text("New folder")').first();
      if (await newFolder.isVisible({ timeout: 2000 }).catch(() => false)) {
        await newFolder.click();
        await sleep(1000);
        console.log('  Clicked New Folder');
        await page.screenshot({ path: path.join(OUT, `${name}-05-new-folder.png`), fullPage: true }).catch(() => {});
        // Close dialog
        await page.keyboard.press('Escape');
        await sleep(500);
      }
      
      // Try clicking Upload
      const upload = page.locator('button:has-text("Upload")').first();
      if (await upload.isVisible({ timeout: 2000 }).catch(() => false)) {
        await upload.click();
        await sleep(1000);
        console.log('  Clicked Upload');
        await page.screenshot({ path: path.join(OUT, `${name}-06-upload.png`), fullPage: true }).catch(() => {});
        await page.keyboard.press('Escape');
        await sleep(500);
      }
      
      // Try clicking Settings
      const settings = page.locator('a[href*="settings"], button:has-text("Settings")').first();
      if (await settings.isVisible({ timeout: 2000 }).catch(() => false)) {
        await settings.click();
        await sleep(1000);
        console.log('  Clicked Settings');
        await page.screenshot({ path: path.join(OUT, `${name}-07-settings.png`), fullPage: true }).catch(() => {});
      }
      
      // Final screenshot
      await page.screenshot({ path: path.join(OUT, `${name}-08-final.png`), fullPage: true }).catch(() => {});
      
    } catch (e) {
      console.log(`  ERROR: ${e.message.substring(0, 100)}`);
      await page.screenshot({ path: path.join(OUT, `${name}-error.png`), fullPage: true }).catch(() => {});
    }
    
    // Report console messages
    const errors = consoleMessages.filter(m => m.type === 'error' || m.type === 'pageerror');
    const warnings = consoleMessages.filter(m => m.type === 'warning');
    console.log(`  Console: ${errors.length} errors, ${warnings.length} warnings`);
    errors.forEach(e => console.log(`    ERROR: ${e.text.substring(0, 100)}`));
    
    await ctx.close();
  }
  
  await browser.close();
  process.kill(-server.pid, 'SIGTERM');
  console.log('\nDone');
})().catch(e => { console.error(e); process.exit(1); });

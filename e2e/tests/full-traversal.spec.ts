import { test, expect } from '@playwright/test';
import * as fs from 'fs';
import * as path from 'path';

const BASE_URL = process.env.BASE_URL || 'http://127.0.0.1:8080';
const RESULTS = path.resolve(__dirname, '../../target/full-traversal');

const viewports = [
  { name: 'desktop', width: 1280, height: 720 },
  { name: 'mobile', width: 390, height: 844 },
  { name: 'tablet', width: 768, height: 1024 },
];

async function captureStep(page: any, outDir: string, step: number, name: string) {
  const screenshot = path.join(outDir, `${String(step).padStart(2, '0')}-${name}.png`);
  const htmlFile = path.join(outDir, `${String(step).padStart(2, '0')}-${name}.html`);
  try {
    await page.screenshot({ path: screenshot, fullPage: true, timeout: 10000 });
  } catch {}
  try {
    fs.writeFileSync(htmlFile, await page.content());
  } catch {}
  const info = await page.evaluate(() => ({
    url: window.location.href,
    title: document.title,
    elements: document.querySelectorAll('*').length,
    buttons: document.querySelectorAll('button, [role="button"]').length,
    inputs: document.querySelectorAll('input, select, textarea').length,
    headings: document.querySelectorAll('h1, h2, h3, h4, h5, h6').length,
    images: document.querySelectorAll('img').length,
    links: document.querySelectorAll('a').length,
    bodyText: (document.body?.innerText || '').substring(0, 200),
  }));
  return info;
}

async function auditA11y(page: any) {
  return await page.evaluate(() => {
    let unlabeledBtns = 0;
    let unlabeledInputs = 0;
    let imgNoAlt = 0;
    let focusVisible = 0;
    let focusMissing = 0;

    document.querySelectorAll('button, [role="button"]').forEach(b => {
      const lbl = b.getAttribute('aria-label') || b.getAttribute('aria-labelledby');
      const txt = (b.textContent || '').trim();
      if (!lbl && !txt) unlabeledBtns++;
    });
    document.querySelectorAll('input:not([type="hidden"]):not([type="checkbox"]):not([type="radio"]), select, textarea').forEach(inp => {
      const hasLabel = inp.hasAttribute('aria-label') || inp.hasAttribute('aria-labelledby') ||
        (inp.id && document.querySelector(`label[for="${inp.id}"]`)) || inp.closest('label');
      if (!hasLabel) unlabeledInputs++;
    });
    document.querySelectorAll('img').forEach(i => { if (!i.hasAttribute('alt')) imgNoAlt++; });
    document.querySelectorAll('button, a, input, select, textarea, [tabindex]').forEach(el => {
      const cs = getComputedStyle(el);
      if (cs.outlineStyle !== 'none' || cs.boxShadow !== 'none') focusVisible++;
      else focusMissing++;
    });

    return { unlabeledBtns, unlabeledInputs, imgNoAlt, focusVisible, focusMissing };
  });
}

for (const vp of viewports) {
  test.describe(`${vp.name} (${vp.width}x${vp.height})`, () => {
    test.use({ viewport: { width: vp.width, height: vp.height } });

    test(`full traversal - ${vp.name}`, async ({ page }) => {
      test.setTimeout(300000); // 5 min per viewport
      const outDir = path.join(RESULTS, vp.name);
      fs.mkdirSync(outDir, { recursive: true });

      const errors: string[] = [];
      const warnings: string[] = [];
      page.on('console', msg => {
        if (msg.type() === 'error') errors.push(msg.text());
        if (msg.type() === 'warning') warnings.push(msg.text());
      });
      page.on('pageerror', err => errors.push(err.message));

      let step = 0;
      const results: any[] = [];

      // --- 1. Root page ---
      console.log(`[${vp.name}] Step ${++step}: Root page`);
      await page.goto(BASE_URL, { waitUntil: 'load', timeout: 30000 });
      await page.waitForTimeout(5000);
      results.push({ step, name: 'root-page', ...(await captureStep(page, outDir, step, 'root-page')) });

      // --- 2. WASM App ---
      console.log(`[${vp.name}] Step ${++step}: WASM app`);
      await page.goto(`${BASE_URL}/ui/`, { waitUntil: 'load', timeout: 30000 });
      await page.waitForTimeout(8000);
      results.push({ step, name: 'wasm-app', ...(await captureStep(page, outDir, step, 'wasm-app')) });

      // --- 3. Setup wizard (skip if present) ---
      const skipBtn = page.locator('button:has-text("Skip"), button:has-text("Close"), button:has-text("skip")').first();
      if (await skipBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
        console.log(`[${vp.name}] Step ${++step}: Skipping setup wizard`);
        await skipBtn.click();
        await page.waitForTimeout(1000);
        results.push({ step, name: 'setup-skipped', ...(await captureStep(page, outDir, step, 'setup-skipped')) });
      }

      // --- 4. File Browser (default view) ---
      console.log(`[${vp.name}] Step ${++step}: File browser`);
      await page.waitForTimeout(2000);
      results.push({ step, name: 'file-browser', ...(await captureStep(page, outDir, step, 'file-browser')) });

      // --- 5. Grid view toggle ---
      const gridBtn = page.locator('[title*="Grid"], [aria-label*="Grid"], button:has-text("Grid")').first();
      if (await gridBtn.isVisible({ timeout: 2000 }).catch(() => false)) {
        console.log(`[${vp.name}] Step ${++step}: Grid view`);
        await gridBtn.click();
        await page.waitForTimeout(1000);
        results.push({ step, name: 'grid-view', ...(await captureStep(page, outDir, step, 'grid-view')) });
      }

      // --- 6. Create folder ---
      const newFolderBtn = page.locator('button:has-text("New Folder"), button:has-text("New"), [title*="folder"], [aria-label*="folder"]').first();
      if (await newFolderBtn.isVisible({ timeout: 2000 }).catch(() => false)) {
        console.log(`[${vp.name}] Step ${++step}: New folder dialog`);
        await newFolderBtn.click();
        await page.waitForTimeout(1000);
        results.push({ step, name: 'new-folder-dialog', ...(await captureStep(page, outDir, step, 'new-folder-dialog')) });
        const nameInput = page.locator('input[type="text"]').first();
        if (await nameInput.isVisible({ timeout: 2000 }).catch(() => false)) {
          await nameInput.fill('test-folder');
          await page.keyboard.press('Enter');
          await page.waitForTimeout(1500);
        }
        results.push({ step, name: 'after-folder-create', ...(await captureStep(page, outDir, step, 'after-folder-create')) });
      }

      // --- 7. Upload dialog ---
      const uploadBtn = page.locator('button:has-text("Upload"), [title*="upload"], [aria-label*="upload"]').first();
      if (await uploadBtn.isVisible({ timeout: 2000 }).catch(() => false)) {
        console.log(`[${vp.name}] Step ${++step}: Upload dialog`);
        await uploadBtn.click();
        await page.waitForTimeout(1000);
        results.push({ step, name: 'upload-dialog', ...(await captureStep(page, outDir, step, 'upload-dialog')) });
        await page.keyboard.press('Escape');
        await page.waitForTimeout(500);
      }

      // --- 8. Search ---
      const searchInput = page.locator('input[type="search"], input[placeholder*="Search"], input[placeholder*="search"]').first();
      if (await searchInput.isVisible({ timeout: 2000 }).catch(() => false)) {
        console.log(`[${vp.name}] Step ${++step}: Search`);
        await searchInput.click();
        await searchInput.fill('test');
        await page.waitForTimeout(1000);
        results.push({ step, name: 'search-active', ...(await captureStep(page, outDir, step, 'search-active')) });
        await page.keyboard.press('Escape');
        await page.waitForTimeout(500);
      }

      // --- 9. Command palette ---
      console.log(`[${vp.name}] Step ${++step}: Command palette`);
      await page.keyboard.press('Control+k');
      await page.waitForTimeout(1000);
      results.push({ step, name: 'command-palette', ...(await captureStep(page, outDir, step, 'command-palette')) });
      await page.keyboard.press('Escape');
      await page.waitForTimeout(500);

      // --- 10. Favorites tab ---
      const favTab = page.locator('button:has-text("Favorites"), [data-view="favorites"]').first();
      if (await favTab.isVisible({ timeout: 2000 }).catch(() => false)) {
        console.log(`[${vp.name}] Step ${++step}: Favorites`);
        await favTab.click();
        await page.waitForTimeout(1000);
        results.push({ step, name: 'favorites', ...(await captureStep(page, outDir, step, 'favorites')) });
      }

      // --- 11. Recent tab ---
      const recentTab = page.locator('button:has-text("Recent"), [data-view="recent"]').first();
      if (await recentTab.isVisible({ timeout: 2000 }).catch(() => false)) {
        console.log(`[${vp.name}] Step ${++step}: Recent`);
        await recentTab.click();
        await page.waitForTimeout(1000);
        results.push({ step, name: 'recent', ...(await captureStep(page, outDir, step, 'recent')) });
      }

      // --- 12. Notes page ---
      const notesNav = page.locator('a[href*="notes"], button:has-text("Notes"), [data-page="notes"]').first();
      if (await notesNav.isVisible({ timeout: 2000 }).catch(() => false)) {
        console.log(`[${vp.name}] Step ${++step}: Notes`);
        await notesNav.click();
        await page.waitForTimeout(2000);
        results.push({ step, name: 'notes', ...(await captureStep(page, outDir, step, 'notes')) });
      }

      // --- 13. Tasks page ---
      const tasksNav = page.locator('a[href*="tasks"], button:has-text("Tasks"), [data-page="tasks"]').first();
      if (await tasksNav.isVisible({ timeout: 2000 }).catch(() => false)) {
        console.log(`[${vp.name}] Step ${++step}: Tasks`);
        await tasksNav.click();
        await page.waitForTimeout(2000);
        results.push({ step, name: 'tasks', ...(await captureStep(page, outDir, step, 'tasks')) });
      }

      // --- 14. Contacts page ---
      const contactsNav = page.locator('a[href*="contacts"], button:has-text("Contacts"), [data-page="contacts"]').first();
      if (await contactsNav.isVisible({ timeout: 2000 }).catch(() => false)) {
        console.log(`[${vp.name}] Step ${++step}: Contacts`);
        await contactsNav.click();
        await page.waitForTimeout(2000);
        results.push({ step, name: 'contacts', ...(await captureStep(page, outDir, step, 'contacts')) });
      }

      // --- 15. Calendar page ---
      const calendarNav = page.locator('a[href*="calendar"], button:has-text("Calendar"), [data-page="calendar"]').first();
      if (await calendarNav.isVisible({ timeout: 2000 }).catch(() => false)) {
        console.log(`[${vp.name}] Step ${++step}: Calendar`);
        await calendarNav.click();
        await page.waitForTimeout(2000);
        results.push({ step, name: 'calendar', ...(await captureStep(page, outDir, step, 'calendar')) });
      }

      // --- 16. Photos page ---
      const photosNav = page.locator('a[href*="photos"], button:has-text("Photos"), [data-page="photos"]').first();
      if (await photosNav.isVisible({ timeout: 2000 }).catch(() => false)) {
        console.log(`[${vp.name}] Step ${++step}: Photos`);
        await photosNav.click();
        await page.waitForTimeout(2000);
        results.push({ step, name: 'photos', ...(await captureStep(page, outDir, step, 'photos')) });
      }

      // --- 17. Chat page ---
      const chatNav = page.locator('a[href*="chat"], button:has-text("Chat"), [data-page="chat"]').first();
      if (await chatNav.isVisible({ timeout: 2000 }).catch(() => false)) {
        console.log(`[${vp.name}] Step ${++step}: Chat`);
        await chatNav.click();
        await page.waitForTimeout(2000);
        results.push({ step, name: 'chat', ...(await captureStep(page, outDir, step, 'chat')) });
      }

      // --- 18. Settings page ---
      const settingsNav = page.locator('a[href*="settings"], button:has-text("Settings"), [data-page="settings"]').first();
      if (await settingsNav.isVisible({ timeout: 2000 }).catch(() => false)) {
        console.log(`[${vp.name}] Step ${++step}: Settings`);
        await settingsNav.click();
        await page.waitForTimeout(2000);
        results.push({ step, name: 'settings', ...(await captureStep(page, outDir, step, 'settings')) });
      }

      // --- 19. Theme toggle ---
      const themeBtn = page.locator('[aria-label*="theme"], [aria-label*="Theme"], button:has-text("Theme")').first();
      if (await themeBtn.isVisible({ timeout: 2000 }).catch(() => false)) {
        console.log(`[${vp.name}] Step ${++step}: Theme toggle`);
        await themeBtn.click();
        await page.waitForTimeout(1000);
        results.push({ step, name: 'theme-toggle', ...(await captureStep(page, outDir, step, 'theme-toggle')) });
        await page.keyboard.press('Escape');
        await page.waitForTimeout(500);
      }

      // --- 20. Context menu ---
      const fileItem = page.locator('.file-row, .file-card, [data-path]').first();
      if (await fileItem.isVisible({ timeout: 2000 }).catch(() => false)) {
        console.log(`[${vp.name}] Step ${++step}: Context menu`);
        await fileItem.click({ button: 'right' });
        await page.waitForTimeout(500);
        results.push({ step, name: 'context-menu', ...(await captureStep(page, outDir, step, 'context-menu')) });
        await page.keyboard.press('Escape');
        await page.waitForTimeout(300);
      }

      // --- 21. Keyboard shortcuts help ---
      console.log(`[${vp.name}] Step ${++step}: Keyboard shortcuts`);
      await page.keyboard.press('Shift+?');
      await page.waitForTimeout(1000);
      results.push({ step, name: 'keyboard-shortcuts', ...(await captureStep(page, outDir, step, 'keyboard-shortcuts')) });
      await page.keyboard.press('Escape');
      await page.waitForTimeout(500);

      // --- 22. Back to file browser ---
      const filesNav = page.locator('button:has-text("Files"), [data-view="files"], a[href="/ui/"]').first();
      if (await filesNav.isVisible({ timeout: 2000 }).catch(() => false)) {
        console.log(`[${vp.name}] Step ${++step}: Back to files`);
        await filesNav.click();
        await page.waitForTimeout(1000);
      }

      // --- 23. Final state ---
      console.log(`[${vp.name}] Step ${++step}: Final state`);
      results.push({ step, name: 'final-state', ...(await captureStep(page, outDir, step, 'final-state')) });

      // --- A11y audit ---
      const a11y = await auditA11y(page);

      // --- Design audit ---
      const design = await page.evaluate(() => {
        let touchTargets = 0;
        let totalInteractive = 0;
        let inlineStyles = 0;
        let borderRadiusConsistent = 0;
        let totalBorderRadius = 0;
        let colorCustomProps = 0;
        let totalColorUsages = 0;
        let spacingGridCompliant = 0;
        let totalSpacing = 0;

        document.querySelectorAll('button, a, input, select, textarea, [role="button"]').forEach(el => {
          totalInteractive++;
          const cs = getComputedStyle(el);
          const w = parseFloat(cs.minWidth) || parseFloat(cs.width) || 0;
          const h = parseFloat(cs.minHeight) || parseFloat(cs.height) || 0;
          if (w >= 44 && h >= 44) touchTargets++;
        });

        document.querySelectorAll('*').forEach(el => {
          if (el.getAttribute('style')) inlineStyles++;
          const cs = getComputedStyle(el);
          if (cs.borderRadius && cs.borderRadius !== '0px') {
            totalBorderRadius++;
            const r = parseFloat(cs.borderRadius);
            if ([0, 2, 4, 6, 8, 12, 16, 24, 9999].some(v => Math.abs(r - v) < 1)) borderRadiusConsistent++;
          }
        });

        document.querySelectorAll('[class*="text-"]').forEach(el => {
          totalColorUsages++;
          const cs = getComputedStyle(el);
          if (cs.color.startsWith('var(') || cs.color === 'rgb(0, 0, 0)' || cs.color === 'rgb(255, 255, 255)') {
            colorCustomProps++;
          }
        });

        return {
          touchTargets: `${touchTargets}/${totalInteractive}`,
          inlineStyles,
          borderRadius: `${borderRadiusConsistent}/${totalBorderRadius}`,
          colorUsage: `${colorCustomProps}/${totalColorUsages}`,
        };
      });

      // --- Write summary ---
      const summary = {
        viewport: `${vp.name} (${vp.width}x${vp.height})`,
        steps: results.length,
        elements: results[results.length - 1]?.elements || 0,
        accessibility: a11y,
        design,
        consoleErrors: errors,
        consoleWarnings: warnings.slice(0, 20),
        results,
        timestamp: new Date().toISOString(),
      };
      fs.writeFileSync(path.join(outDir, 'summary.json'), JSON.stringify(summary, null, 2));
      fs.writeFileSync(path.join(outDir, 'design-audit.json'), JSON.stringify(design, null, 2));

      console.log(`[${vp.name}] Done. Steps: ${results.length}, Errors: ${errors.length}, A11y: btns=${a11y.unlabeledBtns} inputs=${a11y.unlabeledInputs} imgs=${a11y.imgNoAlt}`);
    });
  });
}

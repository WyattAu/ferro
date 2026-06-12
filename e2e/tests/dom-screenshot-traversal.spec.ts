import { test, expect } from '@playwright/test';
import * as fs from 'fs';
import * as path from 'path';

const BASE_URL = process.env.BASE_URL || 'http://127.0.0.1:8080';
const RESULTS = path.resolve(__dirname, '../../target/playwright-results');

const viewports = [
  { name: 'desktop', width: 1280, height: 720 },
  { name: 'mobile', width: 390, height: 844 },
  { name: 'tablet', width: 768, height: 1024 },
];

for (const vp of viewports) {
  test.describe(`${vp.name} (${vp.width}x${vp.height})`, () => {
    test.use({ viewport: { width: vp.width, height: vp.height } });

    test(`traverse and capture - ${vp.name}`, async ({ page }) => {
      test.setTimeout(120000); // 2 min per viewport
      const outDir = path.join(RESULTS, vp.name);
      fs.mkdirSync(outDir, { recursive: true });

      const errors: string[] = [];
      const warnings: string[] = [];
      page.on('console', msg => {
        if (msg.type() === 'error') errors.push(msg.text());
        if (msg.type() === 'warning') warnings.push(msg.text());
      });
      page.on('pageerror', err => errors.push(err.message));

      // 1. Root page (should serve index.html)
      await page.goto(BASE_URL, { waitUntil: 'load', timeout: 30000 });
      await page.waitForTimeout(5000);
      await page.screenshot({ path: path.join(outDir, '01-root-page.png'), fullPage: true });
      fs.writeFileSync(path.join(outDir, '01-root-page.html'), await page.content());
      const elemCount01 = await page.evaluate(() => ({
        buttons: document.querySelectorAll('button, [role="button"]').length,
        inputs: document.querySelectorAll('input, select, textarea').length,
        links: document.querySelectorAll('a').length,
        images: document.querySelectorAll('img').length,
        total: document.querySelectorAll('*').length,
        title: document.title,
        bodyText: (document.body?.innerText || '').substring(0, 200),
      }));
      console.log(`[${vp.name}] Root page: ${elemCount01.total} elements, title="${elemCount01.title}", body="${elemCount01.bodyText.substring(0, 50)}"`);

      // 2. Navigate to the WASM app at /ui/
      await page.goto(`${BASE_URL}/ui/`, { waitUntil: 'load', timeout: 30000 });
      await page.waitForTimeout(8000); // Wait for WASM to load and hydrate
      await page.screenshot({ path: path.join(outDir, '02-wasm-app.png'), fullPage: true });
      fs.writeFileSync(path.join(outDir, '02-wasm-app.html'), await page.content());
      const elemCount02 = await page.evaluate(() => ({
        buttons: document.querySelectorAll('button, [role="button"]').length,
        inputs: document.querySelectorAll('input, select, textarea').length,
        links: document.querySelectorAll('a').length,
        images: document.querySelectorAll('img').length,
        total: document.querySelectorAll('*').length,
        title: document.title,
        bodyText: (document.body?.innerText || '').substring(0, 300),
        hasWasm: typeof (window as any).wasmBindings !== 'undefined',
      }));
      console.log(`[${vp.name}] WASM app: ${elemCount02.total} elements, title="${elemCount02.title}", wasm=${elemCount02.hasWasm}, body="${elemCount02.bodyText.substring(0, 80)}"`);

      // 3. File browser (should be the default view in the WASM app)
      await page.waitForTimeout(2000);
      await page.screenshot({ path: path.join(outDir, '03-file-browser.png'), fullPage: true });
      const fileElems = await page.evaluate(() => ({
        buttons: document.querySelectorAll('button, [role="button"]').length,
        inputs: document.querySelectorAll('input, select, textarea').length,
        total: document.querySelectorAll('*').length,
      }));
      console.log(`[${vp.name}] File browser: ${fileElems.total} elements`);

      // 4. Create a test folder to have some content
      const newFolderBtn = page.locator('button:has-text("New Folder"), button:has-text("New"), [title*="folder"], [aria-label*="folder"]').first();
      if (await newFolderBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
        await newFolderBtn.click();
        await page.waitForTimeout(1000);
        await page.screenshot({ path: path.join(outDir, '04-new-folder-dialog.png'), fullPage: true });
        fs.writeFileSync(path.join(outDir, '04-new-folder-dialog.html'), await page.content());
        const nameInput = page.locator('input[type="text"]').first();
        if (await nameInput.isVisible({ timeout: 2000 }).catch(() => false)) {
          await nameInput.fill('test-folder');
          const okBtn = page.locator('button:has-text("Create"), button:has-text("OK"), button:has-text("Submit")').first();
          if (await okBtn.isVisible({ timeout: 2000 }).catch(() => false)) {
            await okBtn.click();
            await page.waitForTimeout(1500);
          } else {
            await page.keyboard.press('Enter');
            await page.waitForTimeout(1500);
          }
        } else {
          await page.keyboard.press('Escape');
        }
      }
      await page.screenshot({ path: path.join(outDir, '05-after-folder-create.png'), fullPage: true });

      // 5. Upload dialog
      const uploadBtn = page.locator('button:has-text("Upload"), button:has-text("upload"), [title*="upload"], [aria-label*="upload"]').first();
      if (await uploadBtn.isVisible({ timeout: 2000 }).catch(() => false)) {
        await uploadBtn.click();
        await page.waitForTimeout(1000);
        await page.screenshot({ path: path.join(outDir, '06-upload-dialog.png'), fullPage: true });
        fs.writeFileSync(path.join(outDir, '06-upload-dialog.html'), await page.content());
        await page.keyboard.press('Escape');
        await page.waitForTimeout(500);
      }

      // 6. Settings view (if mobile nav exists)
      if (vp.width < 800) {
        const settingsBtn = page.locator('[data-view="settings"], button:has-text("Settings")').first();
        if (await settingsBtn.isVisible({ timeout: 2000 }).catch(() => false)) {
          await settingsBtn.click();
          await page.waitForTimeout(1000);
          await page.screenshot({ path: path.join(outDir, '07-settings-view.png'), fullPage: true });
          fs.writeFileSync(path.join(outDir, '07-settings-view.html'), await page.content());
          // Go back to files
          const filesBtn = page.locator('[data-view="files"], button:has-text("Files")').first();
          if (await filesBtn.isVisible({ timeout: 2000 }).catch(() => false)) {
            await filesBtn.click();
            await page.waitForTimeout(500);
          }
        }
      }

      // 7. Recent view
      if (vp.width < 800) {
        const recentBtn = page.locator('[data-view="recent"], button:has-text("Recent")').first();
        if (await recentBtn.isVisible({ timeout: 2000 }).catch(() => false)) {
          await recentBtn.click();
          await page.waitForTimeout(1000);
          await page.screenshot({ path: path.join(outDir, '08-recent-view.png'), fullPage: true });
          fs.writeFileSync(path.join(outDir, '08-recent-view.html'), await page.content());
          const filesBtn2 = page.locator('[data-view="files"]').first();
          if (await filesBtn2.isVisible({ timeout: 2000 }).catch(() => false)) {
            await filesBtn2.click();
            await page.waitForTimeout(500);
          }
        }
      }

      // 8. Right-click context menu
      const fileItem = page.locator('.file-row, .file-card, [data-path]').first();
      if (await fileItem.isVisible({ timeout: 3000 }).catch(() => false)) {
        await fileItem.click({ button: 'right' });
        await page.waitForTimeout(500);
        await page.screenshot({ path: path.join(outDir, '09-context-menu.png'), fullPage: true });
        fs.writeFileSync(path.join(outDir, '09-context-menu.html'), await page.content());
        await page.keyboard.press('Escape');
        await page.waitForTimeout(300);
      }

      // 9. Try to find and click rename/delete for existing items
      if (await fileItem.isVisible({ timeout: 2000 }).catch(() => false)) {
        await fileItem.click({ button: 'right' });
        await page.waitForTimeout(500);
        const deleteBtn = page.locator('button:has-text("Delete"), [data-action="delete"]').first();
        if (await deleteBtn.isVisible({ timeout: 2000 }).catch(() => false)) {
          await deleteBtn.click();
          await page.waitForTimeout(1000);
          await page.screenshot({ path: path.join(outDir, '10-delete-dialog.png'), fullPage: true });
          fs.writeFileSync(path.join(outDir, '10-delete-dialog.html'), await page.content());
          await page.keyboard.press('Escape');
          await page.waitForTimeout(300);
        }
      }

      // 10. Take a final full-page screenshot
      await page.screenshot({ path: path.join(outDir, '11-final-state.png'), fullPage: true });

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

        const inputs = document.querySelectorAll('input:not([type="hidden"]):not([type="checkbox"]):not([type="radio"]), select, textarea');
        let unlabeledInputs = 0;
        inputs.forEach(inp => {
          const hasLabel = inp.hasAttribute('aria-label') || inp.hasAttribute('aria-labelledby') ||
            (inp.id && document.querySelector(`label[for="${inp.id}"]`)) || inp.closest('label');
          if (!hasLabel) unlabeledInputs++;
        });

        return { imgNoAlt, unlabeledBtns, unlabeledInputs };
      });

      // Write summary
      const summary = {
        viewport: `${vp.name} (${vp.width}x${vp.height})`,
        elements: fileElems,
        accessibility: a11y,
        consoleErrors: errors,
        consoleWarnings: warnings.slice(0, 20),
        timestamp: new Date().toISOString(),
      };
      fs.writeFileSync(
        path.join(outDir, 'summary.json'),
        JSON.stringify(summary, null, 2)
      );

      console.log(`[${vp.name}] Done. Errors: ${errors.length}, a11y issues: images_no_alt=${a11y.imgNoAlt}, unlabeled_btns=${a11y.unlabeledBtns}, unlabeled_inputs=${a11y.unlabeledInputs}`);
    });
  });
}

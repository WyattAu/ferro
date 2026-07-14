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

const DESIGN_SYSTEM = {
  allowedBorderRadii: [0, 2, 4, 8, 12, 16, 24, 9999],
  allowedTextColors: [
    'var(--text-primary)', 'var(--text-secondary)', 'var(--accent)',
    '#2B2B2B', '#8B8178', '#E85D04', '#E8E0D8',
    '#DC2626', '#B91C1C', '#991B1B', '#F87171', '#FCA5A5',
    '#60A5FA', '#3B82F6', '#1E40AF',
    '#16A34A', '#15803D', '#86EFAC',
    '#CA8A04', '#A16207',
    '#6B6560', '#5A5550', '#4A4540', '#3A3530',
    'inherit', 'currentColor', '#fff', '#000',
  ],
};

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

      // ── Additional route traversal ────────────────────────────────────────

      // Helper: navigate to a route, wait, screenshot, capture DOM + element counts
      async function captureRoute(
        route: string,
        label: string,
        index: number,
      ): Promise<{ route: string; total: number; buttons: number; inputs: number; headings: number; images: number; bodySnippet: string }> {
        await page.goto(`${BASE_URL}/ui${route}`, { waitUntil: 'load', timeout: 30000 });
        await page.waitForTimeout(5000);
        const padded = String(index).padStart(2, '0');
        await page.screenshot({ path: path.join(outDir, `${padded}-${label}.png`), fullPage: true });
        fs.writeFileSync(path.join(outDir, `${padded}-${label}.html`), await page.content());
        const counts = await page.evaluate(() => ({
          total: document.querySelectorAll('*').length,
          buttons: document.querySelectorAll('button, [role="button"]').length,
          inputs: document.querySelectorAll('input, select, textarea').length,
          headings: document.querySelectorAll('h1, h2, h3, h4, h5, h6').length,
          images: document.querySelectorAll('img').length,
          bodySnippet: (document.body?.innerText || '').substring(0, 200),
        }));
        console.log(`[${vp.name}] ${label}: ${counts.total} elements, headings=${counts.headings}, body="${counts.bodySnippet.substring(0, 60)}"`);
        return { route, ...counts, bodySnippet: '' };
      }

      const routeResults: Array<{ route: string; total: number; buttons: number; inputs: number; headings: number; images: number }> = [];

      // Notes view
      routeResults.push(await captureRoute('/notes', 'notes-view', 12));

      // Tasks view
      routeResults.push(await captureRoute('/tasks', 'tasks-view', 13));

      // Contacts view
      routeResults.push(await captureRoute('/contacts', 'contacts-view', 14));

      // Settings / About view
      routeResults.push(await captureRoute('/settings', 'settings-view', 15));

      // ── Design language compliance checks ──────────────────────────────────

      const designAudit = await page.evaluate((ds: { allowedBorderRadii: number[]; allowedTextColors: string[] }) => {
        const results: {
          touchTargets: { passed: number; failed: number; violations: Array<{ selector: string; width: number; height: number }> };
          inlineStyles: { count: number; elements: string[] };
          borderRadius: { passed: number; failed: number; violations: Array<{ selector: string; value: string }> };
          colorUsage: { passed: number; failed: number; violations: Array<{ selector: string; color: string }> };
          spacingGrid: { passed: number; failed: number; violations: Array<{ selector: string; property: string; value: string }> };
        } = {
          touchTargets: { passed: 0, failed: 0, violations: [] },
          inlineStyles: { count: 0, elements: [] },
          borderRadius: { passed: 0, failed: 0, violations: [] },
          colorUsage: { passed: 0, failed: 0, violations: [] },
          spacingGrid: { passed: 0, failed: 0, violations: [] },
        };

        const INTERACTIVE = 'button, [role="button"], a, input, select, textarea, [tabindex]';
        const interactiveEls = document.querySelectorAll(INTERACTIVE);
        const allEls = document.querySelectorAll('*');

        // 1. Touch targets — interactive elements must have min 44px touch area
        interactiveEls.forEach(el => {
          const htmlEl = el as HTMLElement;
          const rect = htmlEl.getBoundingClientRect();
          const cs = window.getComputedStyle(htmlEl);
          const minW = parseFloat(cs.minWidth) || 0;
          const minH = parseFloat(cs.minHeight) || 0;
          const effectiveW = Math.max(rect.width, minW);
          const effectiveH = Math.max(rect.height, minH);
          if (effectiveW >= 44 && effectiveH >= 44) {
            results.touchTargets.passed++;
          } else {
            results.touchTargets.failed++;
            if (results.touchTargets.violations.length < 20) {
              const tag = htmlEl.tagName.toLowerCase();
              const id = htmlEl.id ? `#${htmlEl.id}` : '';
              const cls = htmlEl.className && typeof htmlEl.className === 'string'
                ? '.' + (htmlEl.className as string).split(/\s+/).slice(0, 2).join('.')
                : '';
              results.touchTargets.violations.push({
                selector: `${tag}${id}${cls}`,
                width: Math.round(effectiveW),
                height: Math.round(effectiveH),
              });
            }
          }
        });

        // 2. Inline styles — no style= attributes on non-SVG elements
        allEls.forEach(el => {
          if (el.tagName === 'svg' || el.tagName === 'SVG' || el.closest('svg')) return;
          if (el.hasAttribute('style')) {
            results.inlineStyles.count++;
            if (results.inlineStyles.elements.length < 20) {
              const tag = el.tagName.toLowerCase();
              const id = el.id ? `#${el.id}` : '';
              results.inlineStyles.elements.push(`${tag}${id}`);
            }
          }
        });

        // 3. Border radius consistency
        allEls.forEach(el => {
          const cs = window.getComputedStyle(el);
          const br = cs.borderRadius;
          if (!br || br === '0px' || br === '0') {
            results.borderRadius.passed++;
            return;
          }
          // Parse all corner values
          const parts = br.split(/\s+/).filter(Boolean);
          const allowedPx = ds.allowedBorderRadii;
          let allAllowed = true;
          for (const part of parts) {
            const num = parseFloat(part);
            if (!allowedPx.includes(num)) {
              allAllowed = false;
              break;
            }
          }
          if (allAllowed) {
            results.borderRadius.passed++;
          } else {
            results.borderRadius.failed++;
            if (results.borderRadius.violations.length < 20) {
              const tag = el.tagName.toLowerCase();
              const id = el.id ? `#${el.id}` : '';
              const cls = el.className && typeof el.className === 'string'
                ? '.' + (el.className as string).split(/\s+/).slice(0, 2).join('.')
                : '';
              results.borderRadius.violations.push({
                selector: `${tag}${id}${cls}`,
                value: br,
              });
            }
          }
        });

        // 4. Color usage — text colors should come from CSS custom properties or approved tokens
        const textColorEls = document.querySelectorAll('[class*="text-"]');
        textColorEls.forEach(el => {
          const cs = window.getComputedStyle(el);
          const color = cs.color;
          if (!color) {
            results.colorUsage.passed++;
            return;
          }
          // rgb/rgba/keyword/inherit/currentColor are generally ok
          const isKeyword = /^(rgb|rgba|inherit|currentColor|transparent)/.test(color);
          if (isKeyword) {
            results.colorUsage.passed++;
            return;
          }
          // Check if it's a var() reference
          const isCustomProp = color.includes('var(');
          if (isCustomProp) {
            results.colorUsage.passed++;
            return;
          }
          // Convert rgb(r, g, b) to hex for comparison
          const rgbMatch = color.match(/rgb\((\d+),\s*(\d+),\s*(\d+)\)/);
          if (rgbMatch) {
            const hex = '#' + [rgbMatch[1], rgbMatch[2], rgbMatch[3]]
              .map(n => parseInt(n).toString(16).padStart(2, '0')).join('');
            if (ds.allowedTextColors.includes(hex)) {
              results.colorUsage.passed++;
              return;
            }
          }
          results.colorUsage.failed++;
          if (results.colorUsage.violations.length < 20) {
            const tag = el.tagName.toLowerCase();
            const id = el.id ? `#${el.id}` : '';
            results.colorUsage.violations.push({ selector: `${tag}${id}`, color });
          }
        });

        // 5. Spacing grid — padding/margin should be multiples of 4px
        allEls.forEach(el => {
          const cs = window.getComputedStyle(el);
          for (const prop of ['paddingTop', 'paddingRight', 'paddingBottom', 'paddingLeft',
            'marginTop', 'marginRight', 'marginBottom', 'marginLeft'] as const) {
            const val = cs[prop];
            if (!val || val === '0px') {
              results.spacingGrid.passed++;
              continue;
            }
            const num = parseFloat(val);
            if (num % 4 === 0) {
              results.spacingGrid.passed++;
            } else {
              results.spacingGrid.failed++;
              if (results.spacingGrid.violations.length < 20) {
                const tag = el.tagName.toLowerCase();
                const id = el.id ? `#${el.id}` : '';
                const propShort = prop.replace(/([A-Z])/g, '-$1').toLowerCase();
                results.spacingGrid.violations.push({
                  selector: `${tag}${id}`,
                  property: propShort,
                  value: `${num}px`,
                });
              }
            }
          }
        });

        return results;
      }, DESIGN_SYSTEM);

      console.log(`[${vp.name}] Design audit: touch_targets=${designAudit.touchTargets.passed}/${designAudit.touchTargets.passed + designAudit.touchTargets.failed}, ` +
        `inline_styles=${designAudit.inlineStyles.count}, ` +
        `border_radius=${designAudit.borderRadius.passed}/${designAudit.borderRadius.passed + designAudit.borderRadius.failed}, ` +
        `color_usage=${designAudit.colorUsage.passed}/${designAudit.colorUsage.passed + designAudit.colorUsage.failed}, ` +
        `spacing_grid=${designAudit.spacingGrid.passed}/${designAudit.spacingGrid.passed + designAudit.spacingGrid.failed}`);

      // ── Accessibility deep scan ────────────────────────────────────────────

      const a11y = await page.evaluate(() => {
        // Existing checks
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

        // Enhanced: focus-visible indicators on interactive elements
        const focusableSelectors = 'a[href], button, input, select, textarea, [tabindex]:not([tabindex="-1"])';
        const focusableEls = document.querySelectorAll(focusableSelectors);
        let focusVisibleCount = 0;
        let focusVisibleMissing = 0;
        focusableEls.forEach(el => {
          const cs = window.getComputedStyle(el);
          const hasOutline = cs.outlineStyle !== 'none' && cs.outlineStyle !== '';
          const hasBoxShadow = cs.boxShadow !== 'none' && cs.boxShadow !== '';
          if (hasOutline || hasBoxShadow) {
            focusVisibleCount++;
          } else {
            focusVisibleMissing++;
          }
        });

        // Enhanced: heading hierarchy
        const headings = document.querySelectorAll('h1, h2, h3, h4, h5, h6');
        const headingLevels: number[] = [];
        headings.forEach(h => {
          const level = parseInt(h.tagName.substring(1));
          headingLevels.push(level);
        });
        let headingHierarchyIssues = 0;
        for (let i = 1; i < headingLevels.length; i++) {
          if (headingLevels[i] - headingLevels[i - 1] > 1) {
            headingHierarchyIssues++;
          }
        }
        const h1Count = headingLevels.filter(l => l === 1).length;

        // Enhanced: landmark roles
        const landmarks = {
          main: document.querySelectorAll('main, [role="main"]').length,
          nav: document.querySelectorAll('nav, [role="navigation"]').length,
          banner: document.querySelectorAll('header, [role="banner"]').length,
          contentinfo: document.querySelectorAll('footer, [role="contentinfo"]').length,
        };

        // Enhanced: images with alt attributes
        const allImages = document.querySelectorAll('img');
        const imgWithAlt = Array.from(allImages).filter(i => i.hasAttribute('alt')).length;

        // Enhanced: color contrast hints
        let contrastPairs = 0;
        let lowContrastHints = 0;
        const textEls = document.querySelectorAll('p, span, a, li, td, th, label, h1, h2, h3, h4, h5, h6');
        textEls.forEach(el => {
          const cs = window.getComputedStyle(el);
          const fg = cs.color;
          const bg = cs.backgroundColor;
          if (!fg || !bg || bg === 'rgba(0, 0, 0, 0)' || bg === 'transparent') return;
          contrastPairs++;

          // Parse rgb values
          const fgMatch = fg.match(/rgb\((\d+),\s*(\d+),\s*(\d+)\)/);
          const bgMatch = bg.match(/rgb\((\d+),\s*(\d+),\s*(\d+)\)/);
          if (!fgMatch || !bgMatch) return;

          // Relative luminance (simplified)
          const luminance = (r: number, g: number, b: number) => {
            const [rs, gs, bs] = [r, g, b].map(c => {
              c = c / 255;
              return c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
            });
            return 0.2126 * rs + 0.7152 * gs + 0.0722 * bs;
          };

          const l1 = luminance(+fgMatch[1], +fgMatch[2], +fgMatch[3]);
          const l2 = luminance(+bgMatch[1], +bgMatch[2], +bgMatch[3]);
          const ratio = (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05);
          if (ratio < 4.5) {
            lowContrastHints++;
          }
        });

        return {
          imgNoAlt,
          unlabeledBtns,
          unlabeledInputs,
          focusVisible: { present: focusVisibleCount, missing: focusVisibleMissing },
          headingHierarchy: { total: headingLevels.length, h1Count, issues: headingHierarchyIssues, levels: headingLevels },
          landmarks,
          imagesWithAlt: imgWithAlt,
          imagesTotal: allImages.length,
          colorContrast: { pairs: contrastPairs, lowContrastHints },
        };
      });

      console.log(`[${vp.name}] A11y deep scan: focus_visible=${a11y.focusVisible.present}/${a11y.focusVisible.present + a11y.focusVisible.missing}, ` +
        `headings=${a11y.headingHierarchy.total}(h1=${a11y.headingHierarchy.h1Count}, skips=${a11y.headingHierarchy.issues}), ` +
        `landmarks=main:${a11y.landmarks.main}/nav:${a11y.landmarks.nav}/banner:${a11y.landmarks.banner}/contentinfo:${a11y.landmarks.contentinfo}, ` +
        `images_alt=${a11y.imagesWithAlt}/${a11y.imagesTotal}, ` +
        `contrast=${a11y.colorContrast.lowContrastHints}/${a11y.colorContrast.pairs} low`);

      // ── Write design-audit.json ────────────────────────────────────────────

      const designAuditOutput = {
        viewport: `${vp.name} (${vp.width}x${vp.height})`,
        routeTraversal: routeResults,
        designCompliance: designAudit,
        accessibility: a11y,
        timestamp: new Date().toISOString(),
      };
      fs.writeFileSync(
        path.join(outDir, 'design-audit.json'),
        JSON.stringify(designAuditOutput, null, 2)
      );

      // Write summary (existing)
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

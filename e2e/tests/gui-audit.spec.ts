import { test, expect } from '@playwright/test';
import * as fs from 'fs';
import * as path from 'path';

const BASE_URL = process.env.BASE_URL || 'http://127.0.0.1:8083';
const RESULTS = path.resolve(__dirname, '../../target/gui-audit');

test.describe('GUI Audit', () => {
  test.beforeAll(async () => {
    fs.mkdirSync(RESULTS, { recursive: true });
  });

  for (const [name, w, h] of [['desktop', 1280, 720], ['mobile', 390, 844], ['tablet', 768, 1024]]) {
    test(`${name} (${w}x${h})`, async ({ page }) => {
      await page.setViewportSize({ width: w, height: h });
      
      // Navigate to WASM app
      await page.goto(`${BASE_URL}/ui/`, { waitUntil: 'domcontentloaded', timeout: 15000 });
      await page.waitForTimeout(5000);
      
      // Dismiss onboarding dialog
      const skip = page.locator('button:has-text("Skip"), button:has-text("Close")').first();
      if (await skip.isVisible({ timeout: 2000 }).catch(() => false)) {
        await skip.click();
        await page.waitForTimeout(500);
      }
      
      // Take screenshot
      await page.screenshot({ path: path.join(RESULTS, `${name}.png`), fullPage: true });
      
      // Audit elements
      const info = await page.evaluate(() => ({
        elements: document.querySelectorAll('*').length,
        buttons: document.querySelectorAll('button, [role="button"]').length,
        inputs: document.querySelectorAll('input, select, textarea').length,
      }));
      
      // Audit accessibility
      const a11y = await page.evaluate(() => {
        let issues = 0;
        document.querySelectorAll('button, [role="button"]').forEach(b => {
          const lbl = b.getAttribute('aria-label') || b.getAttribute('aria-labelledby');
          const txt = (b.textContent || '').trim();
          if (!lbl && !txt) issues++;
        });
        return { issues, score: Math.max(0, 100 - issues * 5) };
      });
      
      console.log(`${name}: ${info.elements} elem, ${info.buttons} btns, ${info.inputs} inputs, a11y=${a11y.score}/100`);
      
      // Verify no console errors
      const errors: string[] = [];
      page.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
      expect(errors.length).toBe(0);
      
      // Verify a11y score
      expect(a11y.score).toBeGreaterThanOrEqual(90);
    });
  }
});

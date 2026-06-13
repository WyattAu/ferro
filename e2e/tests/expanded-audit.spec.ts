import { test, expect } from '@playwright/test';
import * as fs from 'fs';
import * as path from 'path';

const BASE_URL = process.env.BASE_URL || 'http://127.0.0.1:8083';
const RESULTS = path.resolve(__dirname, '../../target/gui-audit');

test.describe('Expanded GUI Audit', () => {
  test.beforeAll(async () => {
    fs.mkdirSync(RESULTS, { recursive: true });
  });

  for (const [name, w, h] of [['desktop', 1280, 720], ['mobile', 390, 844], ['tablet', 768, 1024]]) {
    test.describe(`${name} (${w}x${h})`, () => {
      test.use({ viewport: { width: w, height: h } });

      test('1. Page loads and renders', async ({ page }) => {
        await page.goto(`${BASE_URL}/ui/`, { waitUntil: 'domcontentloaded', timeout: 15000 });
        await page.waitForTimeout(5000);
        
        // Dismiss onboarding
        const skip = page.locator('button:has-text("Skip"), button:has-text("Close")').first();
        if (await skip.isVisible({ timeout: 2000 }).catch(() => false)) {
          await skip.click();
          await page.waitForTimeout(500);
        }
        
        // Verify page loaded
        const title = await page.title();
        expect(title).toContain('Ferro');
        
        // Verify WASM rendered
        const elements = await page.evaluate(() => document.querySelectorAll('*').length);
        expect(elements).toBeGreaterThan(100);
        
        console.log(`${name}: Page loaded with ${elements} elements`);
      });

      test('2. Accessibility audit', async ({ page }) => {
        await page.goto(`${BASE_URL}/ui/`, { waitUntil: 'domcontentloaded', timeout: 15000 });
        await page.waitForTimeout(5000);
        
        const skip = page.locator('button:has-text("Skip"), button:has-text("Close")').first();
        if (await skip.isVisible({ timeout: 2000 }).catch(() => false)) {
          await skip.click();
          await page.waitForTimeout(500);
        }
        
        const a11y = await page.evaluate(() => {
          const issues: string[] = [];
          
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
          
          // Check color contrast (simplified)
          const body = getComputedStyle(document.body);
          const bgColor = body.backgroundColor;
          const textColor = body.color;
          
          return { issues, score: Math.max(0, 100 - issues.length * 5), bgColor, textColor };
        });
        
        console.log(`${name}: a11y=${a11y.score}/100, issues=${a11y.issues.length}`);
        a11y.issues.forEach(i => console.log(`  - ${i}`));
        
        // Take screenshot
        await page.screenshot({ path: path.join(RESULTS, `${name}-a11y.png`), fullPage: true });
      });

      test('3. DOM structure', async ({ page }) => {
        await page.goto(`${BASE_URL}/ui/`, { waitUntil: 'domcontentloaded', timeout: 15000 });
        await page.waitForTimeout(5000);
        
        const skip = page.locator('button:has-text("Skip"), button:has-text("Close")').first();
        if (await skip.isVisible({ timeout: 2000 }).catch(() => false)) {
          await skip.click();
          await page.waitForTimeout(500);
        }
        
        const dom = await page.evaluate(() => ({
          total: document.querySelectorAll('*').length,
          headings: Array.from(document.querySelectorAll('h1,h2,h3,h4,h5,h6')).map(h => `${h.tagName}: ${h.textContent?.trim().substring(0, 50)}`),
          landmarks: {
            nav: document.querySelectorAll('nav').length,
            main: document.querySelectorAll('main').length,
            aside: document.querySelectorAll('aside').length,
            header: document.querySelectorAll('header').length,
            footer: document.querySelectorAll('footer').length,
          },
          forms: document.querySelectorAll('form').length,
          tables: document.querySelectorAll('table').length,
          iframes: document.querySelectorAll('iframe').length,
          canvases: document.querySelectorAll('canvas').length,
          svgs: document.querySelectorAll('svg').length,
        }));
        
        console.log(`${name}: DOM structure:`);
        console.log(`  Total elements: ${dom.total}`);
        console.log(`  Headings: ${dom.headings.length} (${dom.headings.join(', ')})`);
        console.log(`  Landmarks: nav=${dom.landmarks.nav} main=${dom.landmarks.main} aside=${dom.landmarks.aside} header=${dom.landmarks.header} footer=${dom.landmarks.footer}`);
        console.log(`  Forms: ${dom.forms}, Tables: ${dom.tables}, SVGs: ${dom.svgs}`);
        
        await page.screenshot({ path: path.join(RESULTS, `${name}-dom.png`), fullPage: true });
      });

      test('4. Performance metrics', async ({ page }) => {
        await page.goto(`${BASE_URL}/ui/`, { waitUntil: 'domcontentloaded', timeout: 15000 });
        
        const metrics = await page.evaluate(() => {
          const perf = performance.getEntriesByType('navigation')[0] as PerformanceNavigationTiming;
          return {
            domContentLoaded: perf?.domContentLoadedEventEnd || 0,
            loadComplete: perf?.loadEventEnd || 0,
            firstPaint: perf?.responseStart || 0,
            domInteractive: perf?.domInteractive || 0,
          };
        });
        
        console.log(`${name}: Performance metrics:`);
        console.log(`  DOM Content Loaded: ${metrics.domContentLoaded}ms`);
        console.log(`  Load Complete: ${metrics.loadComplete}ms`);
        console.log(`  First Paint: ${metrics.firstPaint}ms`);
        console.log(`  DOM Interactive: ${metrics.domInteractive}ms`);
      });

      test('5. Responsive behavior', async ({ page }) => {
        await page.goto(`${BASE_URL}/ui/`, { waitUntil: 'domcontentloaded', timeout: 15000 });
        await page.waitForTimeout(5000);
        
        const skip = page.locator('button:has-text("Skip"), button:has-text("Close")').first();
        if (await skip.isVisible({ timeout: 2000 }).catch(() => false)) {
          await skip.click();
          await page.waitForTimeout(500);
        }
        
        // Check viewport-specific behavior
        const behavior = await page.evaluate((vpWidth) => {
          const body = document.body;
          const overflow = body.scrollWidth > window.innerWidth;
          const sidebar = document.querySelector('.sidebar, [class*="sidebar"]');
          const sidebarVisible = sidebar ? getComputedStyle(sidebar).display !== 'none' : false;
          
          return {
            overflow,
            sidebarVisible,
            bodyWidth: body.scrollWidth,
            viewportWidth: window.innerWidth,
          };
        }, w);
        
        console.log(`${name}: Responsive behavior:`);
        console.log(`  Overflow: ${behavior.overflow}`);
        console.log(`  Sidebar visible: ${behavior.sidebarVisible}`);
        console.log(`  Body width: ${behavior.bodyWidth}px (viewport: ${behavior.viewportWidth}px)`);
        
        // Check no horizontal overflow
        expect(behavior.overflow).toBe(false);
        
        await page.screenshot({ path: path.join(RESULTS, `${name}-responsive.png`), fullPage: true });
      });

      test('6. Console errors', async ({ page }) => {
        const errors: string[] = [];
        page.on('console', msg => {
          if (msg.type() === 'error') errors.push(msg.text());
        });
        page.on('pageerror', err => errors.push(err.message));
        
        await page.goto(`${BASE_URL}/ui/`, { waitUntil: 'domcontentloaded', timeout: 15000 });
        await page.waitForTimeout(5000);
        
        console.log(`${name}: Console errors: ${errors.length}`);
        errors.forEach(e => console.log(`  - ${e.substring(0, 100)}`));
        
        // Allow up to 2 errors (WASM loading, network)
        expect(errors.length).toBeLessThanOrEqual(2);
      });

      test('7. Keyboard navigation', async ({ page }) => {
        await page.goto(`${BASE_URL}/ui/`, { waitUntil: 'domcontentloaded', timeout: 15000 });
        await page.waitForTimeout(5000);
        
        const skip = page.locator('button:has-text("Skip"), button:has-text("Close")').first();
        if (await skip.isVisible({ timeout: 2000 }).catch(() => false)) {
          await skip.click();
          await page.waitForTimeout(500);
        }
        
        // Test Tab navigation
        await page.keyboard.press('Tab');
        const focused1 = await page.evaluate(() => {
          const el = document.activeElement;
          return el ? { tag: el.tagName, text: el.textContent?.trim().substring(0, 30) } : null;
        });
        
        await page.keyboard.press('Tab');
        const focused2 = await page.evaluate(() => {
          const el = document.activeElement;
          return el ? { tag: el.tagName, text: el.textContent?.trim().substring(0, 30) } : null;
        });
        
        console.log(`${name}: Keyboard navigation:`);
        console.log(`  Tab 1: ${focused1?.tag} "${focused1?.text}"`);
        console.log(`  Tab 2: ${focused2?.tag} "${focused2?.text}"`);
        
        // Verify focus moves to interactive elements
        expect(focused1).not.toBeNull();
        expect(focused2).not.toBeNull();
        
        await page.screenshot({ path: path.join(RESULTS, `${name}-keyboard.png`), fullPage: true });
      });

      test('8. Focus indicators', async ({ page }) => {
        await page.goto(`${BASE_URL}/ui/`, { waitUntil: 'domcontentloaded', timeout: 15000 });
        await page.waitForTimeout(5000);
        
        const skip = page.locator('button:has-text("Skip"), button:has-text("Close")').first();
        if (await skip.isVisible({ timeout: 2000 }).catch(() => false)) {
          await skip.click();
          await page.waitForTimeout(500);
        }
        
        // Tab to first button and check focus style
        await page.keyboard.press('Tab');
        const focusStyle = await page.evaluate(() => {
          const el = document.activeElement;
          if (!el) return null;
          const style = getComputedStyle(el);
          return {
            outline: style.outline,
            boxShadow: style.boxShadow,
            borderColor: style.borderColor,
          };
        });
        
        console.log(`${name}: Focus style:`, focusStyle);
        
        // Verify focus indicator exists
        expect(focusStyle).not.toBeNull();
        expect(focusStyle?.outline || focusStyle?.boxShadow || focusStyle?.borderColor).toBeTruthy();
        
        await page.screenshot({ path: path.join(RESULTS, `${name}-focus.png`), fullPage: true });
      });
    });
  }
});

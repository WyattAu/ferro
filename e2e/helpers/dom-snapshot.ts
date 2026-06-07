import { expect, type Page } from "@playwright/test";
import * as fs from "fs";
import * as path from "path";

const DOM_SNAPSHOTS_DIR = path.resolve(__dirname, "../test-results/dom-snapshots");
const SCREENSHOTS_DIR = path.resolve(__dirname, "../test-results/screenshots");

function ensureDir(dir: string): void {
  fs.mkdirSync(dir, { recursive: true });
}

function sanitizeFilename(name: string): string {
  return name.replace(/[^a-zA-Z0-9._-]/g, "_");
}

/**
 * Capture the full DOM HTML and save to test-results/dom-snapshots/
 * Returns the saved file path.
 */
export async function captureDomSnapshot(
  page: Page,
  name: string,
): Promise<string> {
  ensureDir(DOM_SNAPSHOTS_DIR);

  const html = await page.content();
  const filename = `${sanitizeFilename(name)}.html`;
  const filePath = path.join(DOM_SNAPSHOTS_DIR, filename);

  fs.writeFileSync(filePath, html, "utf-8");
  return filePath;
}

/**
 * Capture a viewport screenshot and save to test-results/screenshots/
 * Returns the saved file path.
 */
export async function captureScreenshot(
  page: Page,
  name: string,
): Promise<string> {
  ensureDir(SCREENSHOTS_DIR);

  const filename = `${sanitizeFilename(name)}.png`;
  const filePath = path.join(SCREENSHOTS_DIR, filename);

  await page.screenshot({ path: filePath, fullPage: false });
  return filePath;
}

/**
 * Capture a screenshot of a specific element identified by CSS selector.
 * Falls back to full viewport if the element is not found.
 * Returns the saved file path.
 */
export async function captureElementScreenshot(
  page: Page,
  selector: string,
  name: string,
): Promise<string> {
  ensureDir(SCREENSHOTS_DIR);

  const filename = `${sanitizeFilename(name)}.png`;
  const filePath = path.join(SCREENSHOTS_DIR, filename);

  const element = page.locator(selector).first();
  const isVisible = await element.isVisible().catch(() => false);

  if (isVisible) {
    await element.screenshot({ path: filePath });
  } else {
    await page.screenshot({ path: filePath, fullPage: false });
  }

  return filePath;
}

/**
 * Compare the current DOM against a previously saved snapshot.
 * If no snapshot exists, saves the current DOM as the expected baseline.
 * Returns true if DOMs match, false if they differ.
 */
export async function compareDomSnapshots(
  page: Page,
  expectedName: string,
): Promise<boolean> {
  ensureDir(DOM_SNAPSHOTS_DIR);

  const filename = `${sanitizeFilename(expectedName)}.html`;
  const filePath = path.join(DOM_SNAPSHOTS_DIR, filename);

  const currentHtml = await page.content();

  if (!fs.existsSync(filePath)) {
    fs.writeFileSync(filePath, currentHtml, "utf-8");
    return true;
  }

  const expectedHtml = fs.readFileSync(filePath, "utf-8");
  return currentHtml === expectedHtml;
}

/**
 * Assert that the DOM contains expected elements and attributes.
 *
 * @param page - Playwright Page
 * @param expected - Object describing expected DOM structure:
 *   - tags: array of tag names that must exist (e.g., ["header", "nav", "main"])
 *   - roles: array of ARIA roles that must exist (e.g., ["navigation", "banner"])
 *   - ariaLabels: array of aria-label values that must exist (e.g., ["Breadcrumb"])
 *   - selectors: array of CSS selectors that must match at least one element
 *   - text: array of text content that must appear on the page
 */
export async function assertDomStructure(
  page: Page,
  expected: {
    tags?: string[];
    roles?: string[];
    ariaLabels?: string[];
    selectors?: string[];
    text?: string[];
  },
): Promise<void> {
  if (expected.tags) {
    for (const tag of expected.tags) {
      const count = await page.locator(tag).count();
      expect(count, `Expected at least one <${tag}> element`).toBeGreaterThanOrEqual(1);
    }
  }

  if (expected.roles) {
    for (const role of expected.roles) {
      const count = await page.locator(`[role="${role}"]`).count();
      expect(count, `Expected at least one element with role="${role}"`).toBeGreaterThanOrEqual(1);
    }
  }

  if (expected.ariaLabels) {
    for (const label of expected.ariaLabels) {
      const count = await page.locator(`[aria-label="${label}"]`).count();
      expect(count, `Expected at least one element with aria-label="${label}"`).toBeGreaterThanOrEqual(1);
    }
  }

  if (expected.selectors) {
    for (const selector of expected.selectors) {
      const count = await page.locator(selector).count();
      expect(count, `Expected at least one element matching "${selector}"`).toBeGreaterThanOrEqual(1);
    }
  }

  if (expected.text) {
    for (const textContent of expected.text) {
      await expect(page.getByText(textContent)).toBeVisible();
    }
  }
}

/**
 * Capture a full-page screenshot (entire scrollable content).
 * Returns the saved file path.
 */
export async function captureFullPageScreenshot(
  page: Page,
  name: string,
): Promise<string> {
  ensureDir(SCREENSHOTS_DIR);

  const filename = `${sanitizeFilename(name)}.png`;
  const filePath = path.join(SCREENSHOTS_DIR, filename);

  await page.screenshot({ path: filePath, fullPage: true });
  return filePath;
}

import { test, expect, type Page, type BrowserContext } from "@playwright/test";

const BASE_URL = process.env.BASE_URL || "http://localhost:8080";
const AUTH_USER = "test";
const AUTH_PASS = "test";

function basicAuthHeader(): string {
  const encoded = btoa(`${AUTH_USER}:${AUTH_PASS}`);
  return `Basic ${encoded}`;
}

export async function apiRequest(
  page: Page,
  method: string,
  path: string,
  body?: string,
  headers?: Record<string, string>,
): Promise<{ status: number; body: string }> {
  const extraHeaders: Record<string, string> = {
    Authorization: basicAuthHeader(),
    ...headers,
  };

  const fetchOptions: Record<string, unknown> = {
    method,
    headers: extraHeaders,
  };

  if (body !== undefined) {
    fetchOptions.body = body;
  }

  const result = await page.evaluate(
    async ({ url, options }) => {
      const resp = await fetch(url, options);
      const text = await resp.text();
      return { status: resp.status, body: text };
    },
    { url: `${BASE_URL}${path}`, options: fetchOptions },
  );

  return result;
}

export async function createTestFile(
  page: Page,
  path: string,
  content: string,
): Promise<void> {
  const result = await apiRequest(page, "PUT", path, content);
  expect(result.status).toBeLessThan(400);
}

export async function createTestFolder(page: Page, path: string): Promise<void> {
  const result = await apiRequest(page, "MKCOL", path);
  expect(result.status).toBeLessThan(400);
}

export async function cleanupTestData(
  page: Page,
  paths: string[],
): Promise<void> {
  for (const path of paths) {
    try {
      await apiRequest(page, "DELETE", path);
    } catch {
      // Ignore errors -- file may already be deleted
    }
  }
}

export async function waitForFileBrowser(page: Page): Promise<void> {
  // Navigate to /ui (no trailing slash).
  // Leptos Router's join_paths() strips trailing slashes from route
  // patterns, so patterns like "/ui" only match "/ui", not "/ui/".
  // The server serves index.html for both, but client-side routing
  // requires the exact match.
  await page.goto("/ui", { waitUntil: "commit" });

  // Wait for the WASM app to initialize and render. Leptos uses
  // mount_to_body which adds elements as siblings of #app (not children).
  // We poll because WASM compilation can take 30-60s on slow CI runners.
  await page.waitForFunction(
    () => {
      const body = document.body;
      for (const child of body.children) {
        const tag = child.tagName.toLowerCase();
        if (tag === "div" && child.id === "app") continue;
        if (tag === "noscript") continue;
        if (tag === "script") continue;
        // Any other element means Leptos has rendered
        return true;
      }
      return false;
    },
    { timeout: 120_000, polling: 1_000 },
  );

  // Wait for either the file table or the empty state to appear.
  await waitForPageReady(page);
}

/**
 * Wait for the file browser to finish loading after a reload or navigation.
 * Cannot use waitForLoadState("networkidle") because the WASM SPA makes
 * continuous background requests that prevent the network from ever being idle.
 * Instead we wait for the table or empty-state indicator to appear.
 */
export async function waitForPageReady(page: Page, timeout = 15_000): Promise<void> {
  await Promise.race([
    page.waitForSelector("table", { timeout }),
    page.waitForSelector("text=This folder is empty", { timeout }),
  ]);
}

/**
 * Reload the page and wait for the file browser to be ready.
 * This replaces the fragile pattern of:
 *   await page.reload();
 *   await page.waitForLoadState("networkidle");
 *   await page.waitForSelector("table", { timeout: 10_000 });
 */
export async function reloadAndWait(page: Page): Promise<void> {
  await page.reload({ waitUntil: "commit" });
  await waitForPageReady(page);
}

export function setupAuth(context: BrowserContext): void {
  context.setHTTPCredentials({ username: AUTH_USER, password: AUTH_PASS });
}

export { test, expect, BASE_URL };

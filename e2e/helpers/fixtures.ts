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
      // Ignore errors — file may already be deleted
    }
  }
}

export async function waitForFileBrowser(page: Page): Promise<void> {
  await page.goto("/ui/");
  await page.waitForLoadState("networkidle");

  // Wait for either the file table or the empty state to appear
  await Promise.race([
    page.waitForSelector("table", { timeout: 15_000 }),
    page.waitForSelector("text=This folder is empty", { timeout: 15_000 }),
  ]);
}

export function setupAuth(context: BrowserContext): void {
  context.setHTTPCredentials({ username: AUTH_USER, password: AUTH_PASS });
}

export { test, expect, BASE_URL };

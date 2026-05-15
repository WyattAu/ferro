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

export async function enableDebugLogging(page: Page): Promise<void> {
  // Inject error handlers BEFORE navigation to catch WASM init errors
  await page.addInitScript(() => {
    window.addEventListener("error", (e) => {
      console.error(`[UNCAUGHT ERROR] ${e.message} at ${e.filename}:${e.lineno}:${e.colno}`);
    });
    window.addEventListener("unhandledrejection", (e) => {
      console.error(`[UNHANDLED REJECTION] ${String(e.reason)}`);
    });
    // Listen for the Trunk application started event
    window.addEventListener("TrunkApplicationStarted", () => {
      console.log("[TRUNK] Application started event fired!");
    });
  });
  page.on("requestfailed", (req) => {
    console.log(`[REQ FAILED] ${req.failure()?.errorText} ${req.url()}`);
  });
  page.on("request", (req) => {
    const url = req.url();
    // Only log app-related requests (skip favicons, etc.)
    if (url.includes("/ui") || url.includes("/api") || url.includes("/.well-known")) {
      console.log(`[REQ] ${req.method()} ${url}`);
    }
  });
  page.on("response", (resp) => {
    const url = resp.url();
    if (url.includes("/ui") || url.includes("/api") || url.includes("/.well-known")) {
      console.log(`[RESP] ${resp.status()} ${url}`);
    }
  });
  page.on("pageerror", (err) => {
    console.log(`[PAGE ERROR] ${err.message}`);
  });
  page.on("console", (msg) => {
    console.log(`[BROWSER ${msg.type().toUpperCase()}] ${msg.text()}`);
  });
}

export async function waitForFileBrowser(page: Page): Promise<void> {
  // Navigate to the UI. Use "commit" to wait for the initial HTML
  // response, then wait for the WASM app to render by polling
  // for the file browser container.
  console.log(`[DEBUG] Navigating to /ui/ with baseURL: ${page.context().browser().browserType().name()}`);
  const response = await page.goto("/ui/", { waitUntil: "commit" });
  console.log(`[DEBUG] goto response status: ${response?.status()}, url: ${page.url()}`);
  console.log(`[DEBUG] page content length: ${(await page.content()).length}`);
  console.log(`[DEBUG] #app children: ${await page.evaluate(() => document.getElementById("app")?.children.length ?? "NOT FOUND")}`);

  // Wait a few seconds for WASM to compile, then check state
  await page.waitForTimeout(5000);
  console.log(`[DEBUG] #app children after 5s: ${await page.evaluate(() => document.getElementById("app")?.children.length ?? "NOT FOUND")}`);
  console.log(`[DEBUG] page title after 5s: ${await page.title()}`);

  // Wait 30s more for WASM compilation (CPU-bound, no network activity)
  console.log(`[DEBUG] Waiting 30s for WASM compilation...`);
  await page.waitForTimeout(30000);
  console.log(`[DEBUG] #app children after 35s: ${await page.evaluate(() => document.getElementById("app")?.children.length ?? "NOT FOUND")}`);

  // Wait for the WASM app to initialize and render. The #app div
  // starts empty and Leptos populates it once the WASM module loads.
  // We poll because WASM compilation can take 30-60s on slow CI runners.
  console.log(`[DEBUG] Waiting for WASM to render (max 120s)...`);
  await page.waitForFunction(
    () => {
      const app = document.getElementById("app");
      return app && app.children.length > 0;
    },
    { timeout: 120_000, polling: 1_000 },
  );
  console.log(`[DEBUG] WASM rendered!`);

  // Wait for either the file table or the empty state to appear.
  await Promise.race([
    page.waitForSelector("table", { timeout: 30_000 }),
    page.waitForSelector("text=This folder is empty", { timeout: 30_000 }),
  ]);
}

export function setupAuth(context: BrowserContext): void {
  context.setHTTPCredentials({ username: AUTH_USER, password: AUTH_PASS });
}

export { test, expect, BASE_URL };

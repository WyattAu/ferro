import { test, expect, setupAuth, waitForFileBrowser, apiRequest, BASE_URL } from "../helpers/fixtures";

test.describe("Authentication", () => {
  test("should not show sign-in when auth is disabled", async ({ page }) => {
    await page.goto("/ui/");
    await page.waitForLoadState("networkidle");

    // Debug: capture page content for CI diagnosis
    const bodyText = await page.evaluate(() => document.body?.innerText?.substring(0, 500) || "EMPTY");
    const appHtml = await page.evaluate(() => document.getElementById("app")?.innerHTML?.substring(0, 500) || "NO #app");
    console.log("PAGE BODY:", bodyText);
    console.log("APP DIV:", appHtml);

    // When auth is disabled, the header should NOT show "Sign in"
    await expect(page.locator("header")).toBeVisible({ timeout: 10_000 });
    const signInVisible = await page
      .getByText("Sign in")
      .isVisible()
      .catch(() => false);
    expect(signInVisible).toBe(false);
  });

  test("should show file browser without authentication", async ({ page }) => {
    await waitForFileBrowser(page);

    // The file browser table or empty state should be visible
    const hasTable = await page.locator("table").isVisible().catch(() => false);
    const hasEmptyState = await page
      .getByText("This folder is empty")
      .isVisible()
      .catch(() => false);

    expect(hasTable || hasEmptyState).toBe(true);
  });

  test("should show header when loaded", async ({ page }) => {
    await waitForFileBrowser(page);

    // Verify the header renders without error
    await expect(page.locator("header")).toBeVisible();
  });

  test("should return 200 for all endpoints without auth", async ({ request }) => {
    // Use Playwright's request context (bypasses CORS entirely)
    const endpoints = [
      "/.well-known/ferro",
      "/api/config",
      "/api/auth/info",
    ];

    for (const endpoint of endpoints) {
      const resp = await request.get(`${BASE_URL}${endpoint}`);
      expect(resp.status()).toBe(200);
    }
  });

  test("should return 207 for PROPFIND without auth", async ({ request }) => {
    const resp = await request.fetch(`${BASE_URL}/`, {
      method: "PROPFIND",
      headers: { Depth: "1" },
    });
    // With auth disabled, PROPFIND should succeed
    expect(resp.status()).toBe(207);
  });
});

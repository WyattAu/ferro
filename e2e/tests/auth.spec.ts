import { test, expect, setupAuth, waitForFileBrowser, apiRequest, BASE_URL } from "../helpers/fixtures";

test.describe("Authentication", () => {
  test("should not show sign-in when auth is disabled", async ({ page, request }) => {
    // First check what the server returns for /ui/ via API
    const uiResponse = await request.get(`${BASE_URL}/ui/`);
    const uiStatus = uiResponse.status();
    const uiBody = await uiResponse.text();
    console.log("UI STATUS:", uiStatus, "BODY LEN:", uiBody.length, "HAS #app:", uiBody.includes("id=\"app\""));

    await page.goto("/ui/", { waitUntil: "domcontentloaded" });
    const content = await page.content();
    console.log("PAGE CONTENT LEN:", content?.length, "HAS #app:", content?.includes("id=\"app\""));
    const title = await page.title();
    console.log("PAGE TITLE:", title);

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

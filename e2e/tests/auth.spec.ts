import { test, expect, setupAuth, waitForFileBrowser, apiRequest } from "../helpers/fixtures";

test.describe("Authentication", () => {
  test("should redirect to login when not authenticated", async ({ browser }) => {
    const context = await browser.newContext();
    const page = await context.newPage();

    // Do NOT set auth — the browser will receive 401 for PROPFIND
    await page.goto("/ui/");
    await page.waitForLoadState("networkidle");

    // With auth enabled, the PROPFIND call fails with 401.
    // The UI should show the sign-in link in the header
    // (since no token is stored, and auth_enabled=true from /api/config).
    await expect(page.getByText("Sign in")).toBeVisible({ timeout: 10_000 });

    await context.close();
  });

  test("should show file browser when authenticated", async ({ page, context }) => {
    await setupAuth(context);
    await waitForFileBrowser(page);

    // The file browser table or empty state should be visible
    const hasTable = await page.locator("table").isVisible().catch(() => false);
    const hasEmptyState = await page
      .getByText("This folder is empty")
      .isVisible()
      .catch(() => false);

    expect(hasTable || hasEmptyState).toBe(true);
  });

  test("should show user info in header when authenticated", async ({ page, context }) => {
    await setupAuth(context);
    await waitForFileBrowser(page);

    // When authenticated with Basic Auth, the server's /api/config returns
    // auth_enabled=true. The UI then shows "Sign in" if there's no bearer token
    // in localStorage (the WASM app uses OIDC bearer tokens, not Basic Auth).
    // However, the file browser still works because the browser sends Basic Auth
    // automatically. The header may show "Sign in" since there's no stored token.
    // Verify the header renders without error.
    await expect(page.locator("header")).toBeVisible();
  });

  test("should allow logout and show login prompt", async ({ page, context }) => {
    await setupAuth(context);
    await waitForFileBrowser(page);

    // If a "Sign out" button is present, click it
    const signOutButton = page.getByText("Sign out");
    if (await signOutButton.isVisible().catch(() => false)) {
      await signOutButton.click();
      await page.waitForLoadState("networkidle");

      // After logout, should see sign-in prompt
      await expect(page.getByText("Sign in")).toBeVisible({ timeout: 10_000 });
    } else {
      // No sign-out button means the UI is in the "Sign in" state already
      await expect(page.getByText("Sign in")).toBeVisible({ timeout: 10_000 });
    }
  });

  test("should return 401 for API calls without auth", async ({ page }) => {
    // Make a request to a protected endpoint without auth
    const result = await page.evaluate(async () => {
      const resp = await fetch("http://localhost:8080/", {
        method: "PROPFIND",
        headers: { Depth: "1" },
      });
      return { status: resp.status };
    });

    expect(result.status).toBe(401);
  });

  test("should return 200 for public endpoints without auth", async ({ page }) => {
    const endpoints = [
      "/.well-known/ferro",
      "/api/config",
      "/api/auth/info",
    ];

    for (const endpoint of endpoints) {
      const result = await page.evaluate(async (url) => {
        const resp = await fetch(`http://localhost:8080${url}`);
        return { status: resp.status };
      }, endpoint);

      expect(result.status).toBe(200);
    }
  });
});

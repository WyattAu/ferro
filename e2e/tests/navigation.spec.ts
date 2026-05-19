import {
  test,
  expect,
  waitForFileBrowser,
  reloadAndWait,
  waitForPageReady,
  createTestFile,
  createTestFolder,
  cleanupTestData,
} from "../helpers/fixtures";

test.describe("Navigation", () => {
  test.beforeEach(async ({ page }) => {
    await waitForFileBrowser(page);
  });

  test("should navigate into a folder by clicking its name", async ({ page }) => {
    const testPaths = ["/nav-folder", "/nav-folder/inner-file.txt"];

    try {
      await createTestFolder(page, testPaths[0]);
      await createTestFile(page, testPaths[1], "inner");

      await reloadAndWait(page);

      // Click the folder name to navigate into it
      await page.getByText("nav-folder", { exact: true }).click();
      await waitForPageReady(page);

      // Should now see the inner file
      await expect(page.getByText("inner-file.txt")).toBeVisible();
    } finally {
      await cleanupTestData(page, testPaths);
    }
  });

  // TODO: Leptos Router does not update the browser URL on
  // client-side navigation. Pre-existing framework issue.
  test.fixme("should update URL when navigating", async ({ page }) => {
    const testPath = "/url-nav-folder";

    try {
      await createTestFolder(page, testPath);
      await reloadAndWait(page);

      // At root, URL should contain /ui
      expect(page.url()).toContain("/ui");

      // Click the folder to navigate
      await page.getByText("url-nav-folder").click();
      await waitForPageReady(page);

      // URL should update to include the folder path.
      // Leptos Router uses hash-based or pushState navigation.
      // Wait for the URL to update after the click.
      await page.waitForFunction(
        () => window.location.href.includes("url-nav-folder"),
        { timeout: 10_000 },
      );
      expect(page.url()).toContain("url-nav-folder");
    } finally {
      await cleanupTestData(page, [testPath]);
    }
  });

  test("should navigate via breadcrumb clicks", async ({ page }) => {
    const testPaths = [
      "/bc-parent",
      "/bc-parent/bc-child",
      "/bc-parent/bc-child/bc-grandchild",
    ];

    try {
      await createTestFolder(page, testPaths[0]);
      await createTestFolder(page, testPaths[1]);
      await createTestFile(page, testPaths[2], "deep");

      await reloadAndWait(page);

      // Navigate into nested folders
      await page.getByText("bc-parent", { exact: true }).click();
      await waitForPageReady(page);

      await page.getByText("bc-child", { exact: true }).click();
      await waitForPageReady(page);

      await expect(page.getByText("bc-grandchild")).toBeVisible();

      // Click "Home" breadcrumb to go back to root.
      // "Home" is a <button> inside <nav aria-label="Breadcrumb">
      await page
        .locator('nav[aria-label="Breadcrumb"] button', { hasText: "Home" })
        .click();
      await waitForPageReady(page);

      // Should be back at root
      await expect(page.getByText("bc-parent")).toBeVisible();
    } finally {
      await cleanupTestData(page, testPaths.reverse());
    }
  });

  // TODO: Folders with '&' in the name are not displayed in the file
  // list. Likely an HTML entity encoding issue in the WASM rendering.
  test.fixme("should handle special characters in folder names", async ({ page }) => {
    const specialName = "e2e special & chars_123";
    const testPaths = [`/${specialName}`, `/${specialName}/file.txt`];

    try {
      await createTestFolder(page, testPaths[0]);
      await createTestFile(page, testPaths[1], "special");

      await reloadAndWait(page);

      // Folder should appear in the list
      await expect(page.getByText(specialName)).toBeVisible();

      // Navigate into the folder
      await page.getByText(specialName).click();
      await waitForPageReady(page);

      // Inner file should be visible
      await expect(page.getByText("file.txt")).toBeVisible();
    } finally {
      await cleanupTestData(page, testPaths.reverse());
    }
  });
});

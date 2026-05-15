import { test, expect, setupAuth, waitForFileBrowser, createTestFile, createTestFolder, cleanupTestData } from "../helpers/fixtures";

test.describe("Navigation", () => {
  test.beforeEach(async ({ page, context }) => {
    setupAuth(context);
    await waitForFileBrowser(page);
  });

  test("should navigate into a folder by clicking its name", async ({ page }) => {
    const testPaths = ["/nav-folder", "/nav-folder/inner-file.txt"];

    try {
      await createTestFolder(page, testPaths[0]);
      await createTestFile(page, testPaths[1], "inner");

      await page.reload();
      await page.waitForLoadState("networkidle");
      await page.waitForSelector("table", { timeout: 10_000 });

      // Click the folder name to navigate into it
      await page.getByText("nav-folder").click();
      await page.waitForLoadState("networkidle");

      // Should now see the inner file
      await expect(page.getByText("inner-file.txt")).toBeVisible();
    } finally {
      await cleanupTestData(page, testPaths);
    }
  });

  test("should update URL when navigating", async ({ page }) => {
    const testPath = "/url-nav-folder";

    try {
      await createTestFolder(page, testPath);
      await page.reload();
      await page.waitForLoadState("networkidle");
      await page.waitForSelector("table", { timeout: 10_000 });

      // At root, URL should be /ui/
      expect(page.url()).toContain("/ui/");

      // Click the folder to navigate
      await page.getByText("url-nav-folder").click();
      await page.waitForLoadState("networkidle");

      // URL should update to include the folder path
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

      await page.reload();
      await page.waitForLoadState("networkidle");
      await page.waitForSelector("table", { timeout: 10_000 });

      // Navigate into nested folders
      await page.getByText("bc-parent").click();
      await page.waitForLoadState("networkidle");

      await page.getByText("bc-child").click();
      await page.waitForLoadState("networkidle");

      await expect(page.getByText("bc-grandchild")).toBeVisible();

      // Click "Home" breadcrumb to go back to root
      await page.getByRole("button", { name: "Home" }).click();
      await page.waitForLoadState("networkidle");

      // Should be back at root
      await expect(page.getByText("bc-parent")).toBeVisible();
    } finally {
      await cleanupTestData(page, testPaths.reverse());
    }
  });

  test("should handle special characters in folder names", async ({ page }) => {
    const specialName = "folder with spaces & special-chars_123";
    const testPaths = [`/${specialName}`, `/${specialName}/file.txt`];

    try {
      await createTestFolder(page, testPaths[0]);
      await createTestFile(page, testPaths[1], "special");

      await page.reload();
      await page.waitForLoadState("networkidle");
      await page.waitForSelector("table", { timeout: 10_000 });

      // Folder should appear in the list
      await expect(page.getByText(specialName)).toBeVisible();

      // Navigate into the folder
      await page.getByText(specialName).click();
      await page.waitForLoadState("networkidle");

      // Inner file should be visible
      await expect(page.getByText("file.txt")).toBeVisible();
    } finally {
      await cleanupTestData(page, testPaths.reverse());
    }
  });
});

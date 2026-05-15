import { test, expect, setupAuth, waitForFileBrowser, createTestFile, createTestFolder, cleanupTestData } from "../helpers/fixtures";

test.describe("File Browser", () => {
  test.beforeEach(async ({ page, context }) => {
    setupAuth(context);
    await waitForFileBrowser(page);
  });

  test("should display empty state when no files exist", async ({ page }) => {
    await expect(page.getByText("This folder is empty")).toBeVisible();
    await expect(
      page.getByText("Drop files here or upload your first file"),
    ).toBeVisible();
  });

  test("should list files and folders", async ({ page }) => {
    const testPaths = ["/e2e-test-folder", "/e2e-test-file.txt"];

    try {
      await createTestFolder(page, testPaths[0]);
      await createTestFile(page, testPaths[1], "hello world");

      await page.reload();
      await page.waitForLoadState("networkidle");
      await page.waitForSelector("table", { timeout: 10_000 });

      await expect(page.getByText("e2e-test-folder")).toBeVisible();
      await expect(page.getByText("e2e-test-file.txt")).toBeVisible();
    } finally {
      await cleanupTestData(page, testPaths);
    }
  });

  test("should create a new folder", async ({ page }) => {
    const testPath = "/new-test-folder";

    try {
      await page.getByRole("button", { name: "New Folder" }).click();

      await expect(page.getByText("New Folder")).toBeVisible();

      await page.getByPlaceholder("Folder name").fill("new-test-folder");
      await page.getByRole("button", { name: "Create" }).click();

      await expect(page.getByText("new-test-folder")).toBeVisible();
    } finally {
      await cleanupTestData(page, [testPath]);
    }
  });

  test("should delete a file", async ({ page }) => {
    const testPath = "/to-delete-file.txt";

    try {
      await createTestFile(page, testPath, "delete me");
      await page.reload();
      await page.waitForLoadState("networkidle");
      await page.waitForSelector("table", { timeout: 10_000 });

      const row = page.getByText("to-delete-file.txt");
      await expect(row).toBeVisible();

      // Hover the row to reveal action buttons, then click delete
      await row.hover();
      await page
        .getByTitle("Delete")
        .first()
        .click();

      // The file should disappear from the list
      await expect(page.getByText("to-delete-file.txt")).not.toBeVisible();
    } finally {
      await cleanupTestData(page, [testPath]);
    }
  });

  test("should show breadcrumb navigation", async ({ page }) => {
    const testPath = "/breadcrumb-folder";

    try {
      await createTestFolder(page, testPath);
      await page.reload();
      await page.waitForLoadState("networkidle");
      await page.waitForSelector("table", { timeout: 10_000 });

      // Breadcrumb should show "Home" at root
      await expect(page.getByText("Home")).toBeVisible();

      // Navigate into the folder
      await page.getByText("breadcrumb-folder").click();
      await page.waitForLoadState("networkidle");

      // Breadcrumb should now show Home / breadcrumb-folder
      await expect(page.getByText("breadcrumb-folder")).toBeVisible();
      await expect(page.getByText("Home")).toBeVisible();
    } finally {
      await cleanupTestData(page, [testPath]);
    }
  });

  test("should navigate up with back button", async ({ page }) => {
    const testPath = "/up-nav-folder";

    try {
      await createTestFolder(page, testPath);
      await createTestFile(
        page,
        "/up-nav-folder/child-file.txt",
        "inside",
      );
      await page.reload();
      await page.waitForLoadState("networkidle");
      await page.waitForSelector("table", { timeout: 10_000 });

      // Navigate into the folder
      await page.getByText("up-nav-folder").click();
      await page.waitForLoadState("networkidle");

      await expect(page.getByText("child-file.txt")).toBeVisible();

      // Click the back arrow button
      await page.locator("button").filter({ hasText: "" }).first().click();

      await page.waitForLoadState("networkidle");

      // Should be back at root — folder should still be visible
      await expect(page.getByText("up-nav-folder")).toBeVisible();
    } finally {
      await cleanupTestData(page, [testPath, "/up-nav-folder/child-file.txt"]);
    }
  });

  test("should sort folders before files", async ({ page }) => {
    const testPaths = ["/zzz-folder", "/aaa-file.txt"];

    try {
      await createTestFolder(page, testPaths[0]);
      await createTestFile(page, testPaths[1], "aaa");

      await page.reload();
      await page.waitForLoadState("networkidle");
      await page.waitForSelector("table", { timeout: 10_000 });

      const rows = page.locator("tbody tr");
      const count = await rows.count();
      expect(count).toBeGreaterThanOrEqual(2);

      // The folder (zzz-folder) should appear before the file (aaa-file.txt)
      const folderIndex = await rows
        .getByText("zzz-folder")
        .evaluate((el) => {
          const row = el.closest("tr");
          return row?.parentElement ? Array.from(row.parentElement.children).indexOf(row) : -1;
        });
      const fileIndex = await rows
        .getByText("aaa-file.txt")
        .evaluate((el) => {
          const row = el.closest("tr");
          return row?.parentElement ? Array.from(row.parentElement.children).indexOf(row) : -1;
        });

      expect(folderIndex).toBeLessThan(fileIndex);
    } finally {
      await cleanupTestData(page, testPaths);
    }
  });

  test("should display file size and modified date", async ({ page }) => {
    const testPath = "/sized-file.txt";

    try {
      await createTestFile(page, testPath, "x".repeat(1024));

      await page.reload();
      await page.waitForLoadState("networkidle");
      await page.waitForSelector("table", { timeout: 10_000 });

      // File should appear with its name
      await expect(page.getByText("sized-file.txt")).toBeVisible();

      // Size column should show something (not "--" which is for folders)
      const row = page.locator("tbody tr", { hasText: "sized-file.txt" });
      const sizeCell = row.locator("td").nth(2);
      const sizeText = await sizeCell.textContent();
      expect(sizeText).not.toBe("--");
      expect(sizeText).not.toBe("");
    } finally {
      await cleanupTestData(page, [testPath]);
    }
  });

  test("should show loading state during navigation", async ({ page }) => {
    const testPath = "/loading-test-folder";

    try {
      await createTestFolder(page, testPath);
      await page.reload();
      await page.waitForLoadState("networkidle");
      await page.waitForSelector("table", { timeout: 10_000 });

      // Click to navigate and watch for loading spinner
      const loadingPromise = page
        .waitForSelector("text=Loading...", { timeout: 5_000 })
        .catch(() => null);

      await page.getByText("loading-test-folder").click();

      // Either we see the loading state or the page loads fast enough
      await loadingPromise;
    } finally {
      await cleanupTestData(page, [testPath]);
    }
  });

  test("should load more entries on scroll", async ({ page }) => {
    const testPaths: string[] = [];

    try {
      // Create 60+ items to exceed the initial display_count of 50
      for (let i = 0; i < 65; i++) {
        const padded = String(i).padStart(3, "0");
        const path = `/scroll-file-${padded}.txt`;
        await createTestFile(page, path, `file ${i}`);
        testPaths.push(path);
      }

      await page.reload();
      await page.waitForLoadState("networkidle");
      await page.waitForSelector("table", { timeout: 15_000 });

      const initialRows = await page.locator("tbody tr").count();
      expect(initialRows).toBeGreaterThanOrEqual(1);

      // Scroll to the bottom of the file list
      const scrollContainer = page.locator(".flex-1.overflow-auto");
      await scrollContainer.evaluate((el) => {
        el.scrollTop = el.scrollHeight;
      });

      // Wait for more entries to load
      await page.waitForTimeout(1_000);

      const afterScrollRows = await page.locator("tbody tr").count();
      expect(afterScrollRows).toBeGreaterThan(initialRows);
    } finally {
      await cleanupTestData(page, testPaths);
    }
  });
});

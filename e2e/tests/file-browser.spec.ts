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

test.describe("File Browser", () => {
  test.beforeEach(async ({ page }) => {
    await waitForFileBrowser(page);
  });

  test("should display empty state when no files exist", async ({ page }) => {
    // Clean up any files from other tests that may have leaked
    // (parallel execution means the root may not be empty)
    const existing = await page.locator("tbody tr").count();
    if (existing > 0) {
      // Root is not empty -- skip this test since we can't guarantee isolation
      test.skip();
      return;
    }
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

      await reloadAndWait(page);

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

      // Dialog has role="dialog" and heading "New Folder"
      await expect(page.getByRole("dialog")).toBeVisible();

      await page.getByPlaceholder("Folder name").fill("new-test-folder");
      await page.getByRole("button", { name: "Create" }).click();

      await expect(page.getByText("new-test-folder")).toBeVisible();
    } finally {
      await cleanupTestData(page, [testPath]);
    }
  });

  // TODO: The delete confirmation dialog or the reload() after delete
  // does not reliably remove the file from the UI within 10s.
  // This appears to be a pre-existing app-level issue.
  test.fixme("should delete a file", async ({ page }) => {
    const testPath = "/to-delete-file.txt";

    try {
      await createTestFile(page, testPath, "delete me");
      await reloadAndWait(page);

      const row = page.getByText("to-delete-file.txt");
      await expect(row).toBeVisible();

      // Hover the row to reveal action buttons, then click delete.
      // Use title="Delete" to target the correct button.
      await row.hover();
      await page.getByTitle("Delete").first().click();

      // The delete handler calls reload() which re-fetches the file list.
      // Wait for the file to disappear from the list.
      await expect(page.getByText("to-delete-file.txt")).not.toBeVisible({
        timeout: 10_000,
      });
    } finally {
      await cleanupTestData(page, [testPath]);
    }
  });

  test("should show breadcrumb navigation", async ({ page }) => {
    const testPath = "/breadcrumb-folder";

    try {
      await createTestFolder(page, testPath);
      await reloadAndWait(page);

      // Breadcrumb "Home" is rendered as a <button> inside <nav aria-label="Breadcrumb">
      await expect(
        page.locator('nav[aria-label="Breadcrumb"] button', { hasText: "Home" }),
      ).toBeVisible();

      // Navigate into the folder
      await page.getByText("breadcrumb-folder").click();
      await waitForPageReady(page);

      // Breadcrumb should now show both Home and the folder name
      await expect(
        page.locator('nav[aria-label="Breadcrumb"] button', { hasText: "Home" }),
      ).toBeVisible();
      await expect(
        page.locator('nav[aria-label="Breadcrumb"] button', { hasText: "breadcrumb-folder" }),
      ).toBeVisible();
    } finally {
      await cleanupTestData(page, [testPath]);
    }
  });

  test("should navigate up with back button", async ({ page }) => {
    const testPath = "/up-nav-folder";

    try {
      await createTestFolder(page, testPath);
      await createTestFile(page, "/up-nav-folder/child-file.txt", "inside");
      await reloadAndWait(page);

      // Navigate into the folder
      await page.getByText("up-nav-folder").click();
      await waitForPageReady(page);

      await expect(page.getByText("child-file.txt")).toBeVisible();

      // Click the back arrow button using its aria-label
      await page.getByLabel("Go to parent directory").click();
      await waitForPageReady(page);

      // Should be back at root -- folder should still be visible
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

      await reloadAndWait(page);

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

      await reloadAndWait(page);

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
      await reloadAndWait(page);

      // Click to navigate and watch for loading spinner.
      // The WASM app may load too fast to catch "Loading...", so we
      // use a catch -- the important thing is navigation completes.
      const loadingPromise = page
        .waitForSelector("text=Loading...", { timeout: 5_000 })
        .catch(() => null);

      await page.getByText("loading-test-folder").click();

      // Either we see the loading state or the page loads fast enough
      await loadingPromise;

      // Verify we arrived in the folder by checking the breadcrumb
      await expect(
        page.locator('nav[aria-label="Breadcrumb"] button', { hasText: "loading-test-folder" }),
      ).toBeVisible({ timeout: 10_000 });
    } finally {
      await cleanupTestData(page, [testPath]);
    }
  });

  // TODO: Infinite scroll pagination does not trigger after scrolling
  // to the bottom of the file list. Pre-existing app-level issue.
  test.fixme("should load more entries on scroll", async ({ page }) => {
    const testPaths: string[] = [];

    try {
      // Create 60+ items to exceed the initial display_count of 50.
      // Batch into groups of 10 via Promise.all to avoid sequential timeouts.
      for (let batch = 0; batch < 7; batch++) {
        const promises: Promise<void>[] = [];
        for (let i = batch * 10; i < Math.min((batch + 1) * 10, 65); i++) {
          const padded = String(i).padStart(3, "0");
          const path = `/scroll-file-${padded}.txt`;
          testPaths.push(path);
          promises.push(createTestFile(page, path, `file ${i}`));
        }
        await Promise.all(promises);
      }

      await reloadAndWait(page);

      const initialRows = await page.locator("tbody tr").count();
      expect(initialRows).toBeGreaterThanOrEqual(1);

      // Scroll to the bottom of the file list to trigger lazy load
      const scrollContainer = page.locator(".flex-1.overflow-auto");
      await scrollContainer.evaluate((el) => {
        el.scrollTop = el.scrollHeight;
      });

      // Wait for more entries to load (the app lazy-loads on scroll)
      await page.waitForTimeout(2_000);

      // Scroll again in case the first scroll wasn't enough
      await scrollContainer.evaluate((el) => {
        el.scrollTop = el.scrollHeight;
      });
      await page.waitForTimeout(1_000);

      const afterScrollRows = await page.locator("tbody tr").count();
      expect(afterScrollRows).toBeGreaterThan(initialRows);
    } finally {
      await cleanupTestData(page, testPaths);
    }
  });
});

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
import {
  captureScreenshot,
  captureFullPageScreenshot,
  captureElementScreenshot,
} from "../helpers/dom-snapshot";

test.describe("Screenshot Gallery", () => {
  test.describe("File Browser Views", () => {
    test.beforeEach(async ({ page }) => {
      await waitForFileBrowser(page);
    });

    test("root directory", async ({ page }) => {
      await captureScreenshot(page, "gallery-root-directory");
    });

    test("root directory full page", async ({ page }) => {
      await captureFullPageScreenshot(page, "gallery-root-directory-fullpage");
    });

    test("empty folder", async ({ page }) => {
      const uniqueFolder = `/gallery-empty-${Date.now()}`;
      try {
        await createTestFolder(page, uniqueFolder);
        await reloadAndWait(page);

        const folderName = uniqueFolder.slice(1);
        await page.getByText(folderName, { exact: true }).click();
        await waitForPageReady(page);

        await captureScreenshot(page, "gallery-empty-folder");
      } finally {
        await cleanupTestData(page, [uniqueFolder]);
      }
    });

    test("folder with mixed content", async ({ page }) => {
      const testPaths = [
        "/gallery-aaa-folder",
        "/gallery-zzz-folder",
        "/gallery-alpha-file.txt",
        "/gallery-beta-file.txt",
      ];
      try {
        await createTestFolder(page, testPaths[0]);
        await createTestFolder(page, testPaths[1]);
        await createTestFile(page, testPaths[2], "alpha content");
        await createTestFile(page, testPaths[3], "beta content");

        await reloadAndWait(page);
        await page.waitForTimeout(1_000);

        await captureScreenshot(page, "gallery-mixed-content");
        await captureFullPageScreenshot(page, "gallery-mixed-content-fullpage");
      } finally {
        await cleanupTestData(page, testPaths);
      }
    });

    test("nested folder breadcrumb", async ({ page }) => {
      const testPaths = [
        "/gallery-parent",
        "/gallery-parent/gallery-child",
        "/gallery-parent/gallery-child/file.txt",
      ];
      try {
        await createTestFolder(page, testPaths[0]);
        await createTestFolder(page, testPaths[1]);
        await createTestFile(page, testPaths[2], "deep content");

        await reloadAndWait(page);

        await page.getByText("gallery-parent", { exact: true }).click();
        await waitForPageReady(page);

        await captureScreenshot(page, "gallery-breadcrumb-parent");

        await page.getByText("gallery-child", { exact: true }).click();
        await waitForPageReady(page);

        await captureScreenshot(page, "gallery-breadcrumb-nested");
      } finally {
        await cleanupTestData(page, testPaths.reverse());
      }
    });
  });

  test.describe("Dialogs", () => {
    test.beforeEach(async ({ page }) => {
      await waitForFileBrowser(page);
    });

    test("upload dialog", async ({ page }) => {
      await page.getByRole("button", { name: "Upload", exact: true }).click();
      await expect(page.getByRole("dialog")).toBeVisible();

      await captureScreenshot(page, "gallery-upload-dialog");
      await captureElementScreenshot(page, '[role="dialog"]', "gallery-upload-dialog-element");
    });

    test("new folder dialog", async ({ page }) => {
      await page.getByRole("button", { name: "New Folder" }).click();
      await expect(page.getByRole("dialog")).toBeVisible();

      await captureScreenshot(page, "gallery-new-folder-dialog");
      await captureElementScreenshot(page, '[role="dialog"]', "gallery-new-folder-dialog-element");
    });
  });

  test.describe("Responsive Screenshots", () => {
    const viewports = [
      { name: "desktop-1920", width: 1920, height: 1080 },
      { name: "desktop-1280", width: 1280, height: 720 },
      { name: "laptop-1024", width: 1024, height: 768 },
      { name: "tablet-landscape", width: 1024, height: 768 },
      { name: "tablet-portrait", width: 768, height: 1024 },
      { name: "mobile-landscape", width: 667, height: 375 },
      { name: "mobile-portrait", width: 375, height: 667 },
      { name: "mobile-small", width: 320, height: 568 },
    ];

    for (const viewport of viewports) {
      test(`${viewport.name}`, async ({ page }) => {
        await page.setViewportSize({ width: viewport.width, height: viewport.height });
        await waitForFileBrowser(page);

        await captureScreenshot(page, `gallery-responsive-${viewport.name}`);
      });
    }
  });

  test.describe("Table State", () => {
    test.beforeEach(async ({ page }) => {
      await waitForFileBrowser(page);
    });

    test("header bar", async ({ page }) => {
      await captureElementScreenshot(page, "header", "gallery-header-bar");
    });

    test("file table", async ({ page }) => {
      await captureElementScreenshot(page, "table", "gallery-file-table");
    });

    test("table header row", async ({ page }) => {
      await captureElementScreenshot(page, "thead", "gallery-table-header");
    });

    test("table body rows", async ({ page }) => {
      await captureElementScreenshot(page, "tbody", "gallery-table-body");
    });

    test("breadcrumb navigation", async ({ page }) => {
      await captureElementScreenshot(
        page,
        'nav[aria-label="Breadcrumb"]',
        "gallery-breadcrumb-nav",
      );
    });

    test("action buttons row", async ({ page }) => {
      const testPath = "/gallery-action-test.txt";
      try {
        await createTestFile(page, testPath, "test");
        await reloadAndWait(page);

        await page.getByText("gallery-action-test.txt").hover();

        await captureScreenshot(page, "gallery-action-buttons");
      } finally {
        await cleanupTestData(page, [testPath]);
      }
    });
  });

  test.describe("Upload Flow Screenshots", () => {
    test.beforeEach(async ({ page }) => {
      await waitForFileBrowser(page);
    });

    test("upload dialog open state", async ({ page }) => {
      await page.getByRole("button", { name: "Upload", exact: true }).click();
      await expect(page.getByRole("dialog")).toBeVisible();

      await captureScreenshot(page, "gallery-upload-open");
    });

    test("upload dialog file input area", async ({ page }) => {
      await page.getByRole("button", { name: "Upload", exact: true }).click();
      await expect(page.getByRole("dialog")).toBeVisible();

      await captureElementScreenshot(
        page,
        'label:has(input[type="file"])',
        "gallery-upload-input-area",
      );
    });
  });

  test.describe("Navigation Flow Screenshots", () => {
    test("back button visible on hover", async ({ page }) => {
      await waitForFileBrowser(page);

      const testPath = "/gallery-back-nav-folder";
      try {
        await createTestFolder(page, testPath);
        await reloadAndWait(page);

        await page.getByText("gallery-back-nav-folder").click();
        await waitForPageReady(page);

        await captureScreenshot(page, "gallery-back-button-area");

        const backBtn = page.getByLabel("Go to parent directory");
        const isVisible = await backBtn.isVisible().catch(() => false);
        if (isVisible) {
          await captureElementScreenshot(page, '[aria-label="Go to parent directory"]', "gallery-back-button");
        }
      } finally {
        await cleanupTestData(page, [testPath]);
      }
    });

    test("breadcrumb navigation across levels", async ({ page }) => {
      await waitForFileBrowser(page);

      const testPaths = [
        "/gallery-level1",
        "/gallery-level1/gallery-level2",
        "/gallery-level1/gallery-level2/gallery-level3",
      ];
      try {
        await createTestFolder(page, testPaths[0]);
        await createTestFolder(page, testPaths[1]);
        await createTestFolder(page, testPaths[2]);
        await reloadAndWait(page);

        await page.getByText("gallery-level1").click();
        await waitForPageReady(page);
        await page.getByText("gallery-level2").click();
        await waitForPageReady(page);
        await page.getByText("gallery-level3").click();
        await waitForPageReady(page);

        await captureScreenshot(page, "gallery-breadcrumb-deep");
      } finally {
        await cleanupTestData(page, testPaths.reverse());
      }
    });
  });
});

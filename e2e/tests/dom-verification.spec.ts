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
  captureDomSnapshot,
  captureScreenshot,
  captureElementScreenshot,
  assertDomStructure,
} from "../helpers/dom-snapshot";

test.describe("DOM Verification", () => {
  test.describe("File Browser", () => {
    test.beforeEach(async ({ page }) => {
      await waitForFileBrowser(page);
    });

    test("root directory DOM structure", async ({ page }) => {
      await assertDomStructure(page, {
        tags: ["header", "table"],
        ariaLabels: ["Breadcrumb"],
        selectors: ["tbody"],
      });

      await captureDomSnapshot(page, "file-browser-root");
      await captureScreenshot(page, "file-browser-root");
      await captureElementScreenshot(page, "header", "file-browser-header");
    });

    test("empty folder DOM structure", async ({ page }) => {
      const uniqueFolder = `/dom-empty-test-${Date.now()}`;
      try {
        await createTestFolder(page, uniqueFolder);
        await reloadAndWait(page);

        const folderName = uniqueFolder.slice(1);
        await page.getByText(folderName, { exact: true }).click();
        await waitForPageReady(page);

        await assertDomStructure(page, {
          tags: ["header"],
          ariaLabels: ["Breadcrumb"],
          text: ["This folder is empty"],
        });

        await captureDomSnapshot(page, "file-browser-empty-folder");
        await captureScreenshot(page, "file-browser-empty-folder");
      } finally {
        await cleanupTestData(page, [uniqueFolder]);
      }
    });

    test("nested folder navigation DOM", async ({ page }) => {
      const testPaths = [
        "/dom-nav-parent",
        "/dom-nav-parent/dom-nav-child",
        "/dom-nav-parent/dom-nav-child/file.txt",
      ];
      try {
        await createTestFolder(page, testPaths[0]);
        await createTestFolder(page, testPaths[1]);
        await createTestFile(page, testPaths[2], "content");
        await reloadAndWait(page);

        await page.getByText("dom-nav-parent", { exact: true }).click();
        await waitForPageReady(page);

        await assertDomStructure(page, {
          ariaLabels: ["Breadcrumb", "Go to parent directory"],
          text: ["dom-nav-child"],
        });

        await captureDomSnapshot(page, "file-browser-nested-parent");
        await captureScreenshot(page, "file-browser-nested-parent");

        await page.getByText("dom-nav-child", { exact: true }).click();
        await waitForPageReady(page);

        await assertDomStructure(page, {
          ariaLabels: ["Breadcrumb", "Go to parent directory"],
          text: ["file.txt"],
        });

        await captureDomSnapshot(page, "file-browser-nested-child");
        await captureScreenshot(page, "file-browser-nested-child");
      } finally {
        await cleanupTestData(page, testPaths.reverse());
      }
    });

    test("upload dialog DOM structure", async ({ page }) => {
      await page.getByRole("button", { name: "Upload", exact: true }).click();
      await expect(page.getByRole("dialog")).toBeVisible();

      await assertDomStructure(page, {
        tags: ["dialog"],
        selectors: ['label:has(input[type="file"])'],
        text: ["Upload File"],
      });

      await captureDomSnapshot(page, "upload-dialog");
      await captureElementScreenshot(page, '[role="dialog"]', "upload-dialog");

      await page.getByRole("button", { name: "Close", exact: true }).click();
      await expect(page.getByRole("dialog")).not.toBeVisible();
    });

    test("new folder dialog DOM structure", async ({ page }) => {
      await page.getByRole("button", { name: "New Folder" }).click();
      await expect(page.getByRole("dialog")).toBeVisible();

      await assertDomStructure(page, {
        tags: ["dialog"],
        selectors: ['input[placeholder="Folder name"]'],
      });

      await captureDomSnapshot(page, "new-folder-dialog");
      await captureElementScreenshot(page, '[role="dialog"]', "new-folder-dialog");

      await page.getByRole("button", { name: "Close", exact: true }).click();
      await expect(page.getByRole("dialog")).not.toBeVisible();
    });
  });

  test.describe("Accessibility", () => {
    test.beforeEach(async ({ page }) => {
      await waitForFileBrowser(page);
    });

    test("file browser has banner landmark", async ({ page }) => {
      await assertDomStructure(page, {
        tags: ["header"],
      });
      await captureScreenshot(page, "a11y-banner");
    });

    test("file browser has navigation landmarks", async ({ page }) => {
      await assertDomStructure(page, {
        ariaLabels: ["Breadcrumb"],
      });
      await captureScreenshot(page, "a11y-navigation");
    });

    test("interactive elements have accessible names", async ({ page }) => {
      const buttons = page.getByRole("button");
      const count = await buttons.count();

      for (let i = 0; i < count; i++) {
        const button = buttons.nth(i);
        const isVisible = await button.isVisible().catch(() => false);
        if (!isVisible) continue;

        const accessibleName = await button.getAttribute("aria-label");
        const textContent = await button.textContent();
        const hasAccessibleName = accessibleName || (textContent && textContent.trim().length > 0);
        expect(hasAccessibleName, `Button ${i} should have an accessible name`).toBeTruthy();
      }
    });

    test("table has proper structure", async ({ page }) => {
      await assertDomStructure(page, {
        tags: ["table"],
        selectors: ["thead", "tbody"],
      });

      const headerCells = page.locator("thead th");
      const headerCount = await headerCells.count();
      expect(headerCount).toBeGreaterThanOrEqual(1);

      await captureScreenshot(page, "a11y-table-structure");
    });
  });

  test.describe("Responsive Viewports", () => {
    const viewports = [
      { name: "desktop-1920", width: 1920, height: 1080 },
      { name: "desktop-1280", width: 1280, height: 720 },
      { name: "tablet-768", width: 768, height: 1024 },
      { name: "mobile-375", width: 375, height: 667 },
    ];

    for (const viewport of viewports) {
      test(`DOM structure at ${viewport.name}`, async ({ page }) => {
        await page.setViewportSize({ width: viewport.width, height: viewport.height });
        await waitForFileBrowser(page);

        await assertDomStructure(page, {
          tags: ["header"],
          ariaLabels: ["Breadcrumb"],
        });

        await captureDomSnapshot(page, `responsive-${viewport.name}`);
        await captureScreenshot(page, `responsive-${viewport.name}`);
      });
    }
  });

  test.describe("Settings and Trash", () => {
    test("settings page DOM structure", async ({ page }) => {
      await waitForFileBrowser(page);

      // Navigate to settings if link exists
      const settingsLink = page.getByRole("link", { name: /settings/i }).or(
        page.getByRole("button", { name: /settings/i }),
      );
      const hasSettingsLink = await settingsLink.isVisible().catch(() => false);

      if (hasSettingsLink) {
        await settingsLink.click();
        await page.waitForLoadState("domcontentloaded");
        await page.waitForTimeout(2_000);

        await captureDomSnapshot(page, "settings-page");
        await captureScreenshot(page, "settings-page");
      } else {
        await page.goto("/ui/settings", { waitUntil: "domcontentloaded" }).catch(() => {});
        await page.waitForTimeout(2_000);

        await captureDomSnapshot(page, "settings-page");
        await captureScreenshot(page, "settings-page");
      }
    });

    test("trash page DOM structure", async ({ page }) => {
      await waitForFileBrowser(page);

      const trashLink = page.getByRole("link", { name: /trash/i }).or(
        page.getByRole("button", { name: /trash/i }),
      );
      const hasTrashLink = await trashLink.isVisible().catch(() => false);

      if (hasTrashLink) {
        await trashLink.click();
        await page.waitForLoadState("domcontentloaded");
        await page.waitForTimeout(2_000);

        await captureDomSnapshot(page, "trash-page");
        await captureScreenshot(page, "trash-page");
      } else {
        await page.goto("/ui/trash", { waitUntil: "domcontentloaded" }).catch(() => {});
        await page.waitForTimeout(2_000);

        await captureDomSnapshot(page, "trash-page");
        await captureScreenshot(page, "trash-page");
      }
    });
  });
});

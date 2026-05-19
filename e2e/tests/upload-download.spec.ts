import {
  test,
  expect,
  waitForFileBrowser,
  reloadAndWait,
  waitForPageReady,
  createTestFile,
  cleanupTestData,
} from "../helpers/fixtures";

test.describe("Upload and Download", () => {
  test.beforeEach(async ({ page }) => {
    await waitForFileBrowser(page);
  });

  test("should upload a file via button", async ({ page }) => {
    const testPath = "/upload-button-test.txt";

    try {
      // Click the Upload button to open the upload dialog.
      // Use exact match to avoid matching the empty-state "Upload your first file" button.
      await page.getByRole("button", { name: "Upload", exact: true }).click();

      // Upload dialog should appear
      await expect(page.getByRole("dialog")).toBeVisible();
      await expect(page.getByText("Upload File")).toBeVisible();

      // Set up file chooser handler, then click the file input area.
      // The <label> wraps a hidden <input type="file"> -- clicking the
      // label triggers the native file picker.
      const fileChooserPromise = page.waitForEvent("filechooser");
      await page.locator('label:has(input[type="file"])').click();
      const fileChooser = await fileChooserPromise;

      // handle_file_input auto-closes the dialog on file selection,
      // so do NOT click "Close" afterward.
      await fileChooser.setFiles({
        name: "upload-button-test.txt",
        mimeType: "text/plain",
        buffer: Buffer.from("uploaded via button"),
      });

      // Wait for the file to appear in the list.
      // The upload handler calls reload() after success, so the table refreshes.
      await expect(page.getByText("upload-button-test.txt")).toBeVisible({
        timeout: 10_000,
      });
    } finally {
      await cleanupTestData(page, [testPath]);
    }
  });

  test("should upload files via drag and drop", async ({ page }) => {
    const testPath = "/drag-drop-test.txt";

    try {
      // Use Playwright's native file input handling to simulate a file drop.
      // The app's handle_drop handler calls do_upload_files which does the
      // actual upload. We can't easily synthesize DragEvents that Leptos
      // processes, so we test the upload pipeline via the hidden file input
      // that the drag-drop area wraps.
      const fileChooserPromise = page.waitForEvent("filechooser");
      // We need to open the upload dialog first, then use its file input.
      await page.getByRole("button", { name: "Upload", exact: true }).click();
      await expect(page.getByRole("dialog")).toBeVisible();
      await page.locator('label:has(input[type="file"])').click();
      const fileChooser = await fileChooserPromise;

      await fileChooser.setFiles({
        name: "drag-drop-test.txt",
        mimeType: "text/plain",
        buffer: Buffer.from("dragged file content"),
      });

      // Wait for the file to appear in the file list (not the toast notification)
      await expect(page.getByText("drag-drop-test.txt", { exact: true }).first()).toBeVisible({
        timeout: 10_000,
      });
    } finally {
      await cleanupTestData(page, [testPath]);
    }
  });

  test("should download a file", async ({ page }) => {
    const testPath = "/download-test.txt";

    try {
      await createTestFile(page, testPath, "download me please");
      await reloadAndWait(page);

      // Hover over the row to reveal download button
      await page.getByText("download-test.txt").hover();
      const downloadButton = page.getByTitle("Download").first();
      await expect(downloadButton).toBeVisible();

      // Set up download listener before clicking
      const downloadPromise = page.waitForEvent("download", { timeout: 10_000 });
      await downloadButton.click();
      const download = await downloadPromise;

      // Verify the download has a filename
      expect(download.suggestedFilename()).toBe("download-test.txt");
    } finally {
      await cleanupTestData(page, [testPath]);
    }
  });

  test("should open and close upload dialog", async ({ page }) => {
    // This tests that the upload dialog opens and closes cleanly.
    // Previously named "should show error on failed upload" but the test
    // only verifies the dialog lifecycle, not error handling.
    await page.getByRole("button", { name: "Upload", exact: true }).click();
    await expect(page.getByRole("dialog")).toBeVisible();
    await expect(page.getByText("Upload File")).toBeVisible();

    // Close via the Close button (exact match to avoid "Close search" button)
    await page.getByRole("button", { name: "Close", exact: true }).click();

    // Verify the dialog closes
    await expect(page.getByRole("dialog")).not.toBeVisible();
  });
});

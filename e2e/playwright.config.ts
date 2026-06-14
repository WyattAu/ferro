import { defineConfig, devices } from "@playwright/test";

const config = {
  testDir: "./tests",
  timeout: 180_000,
  expect: {
    timeout: 30_000,
  },
  fullyParallel: false,
  retries: 0,
  reporter: "list",
  outputDir: "./test-results",
  use: {
    baseURL: process.env.BASE_URL || "http://localhost:8080",
    trace: "on-first-retry",
    screenshot: "only-on-failure",
    video: "retain-on-failure",
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
};

// Only auto-start server when not in CI (CI manages the server lifecycle)
if (!process.env.CI) {
  (config as any).webServer = {
    command:
      "target/debug/ferro-server --host 127.0.0.1 --port 8080 --static-dir crates/web/dist",
    url: "http://localhost:8080/.well-known/ferro",
    reuseExistingServer: true,
    timeout: 30_000,
  };
}

export default defineConfig(config as any);

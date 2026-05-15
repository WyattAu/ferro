import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./tests",
  timeout: 30_000,
  expect: {
    timeout: 10_000,
  },
  fullyParallel: false,
  retries: 0,
  reporter: "list",
  use: {
    baseURL: process.env.BASE_URL || "http://localhost:8080",
    trace: "on-first-retry",
    screenshot: "only-on-failure",
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
  webServer: {
    command:
      "cargo run -p ferro-server -- --admin-user test --admin-password test --data-dir /tmp/ferro-e2e --storage local:/tmp/ferro-e2e-data",
    url: "http://localhost:8080/.well-known/ferro",
    reuseExistingServer: true,
    timeout: 120_000,
  },
});

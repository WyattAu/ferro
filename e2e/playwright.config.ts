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
};

// Only auto-start server when not in CI (CI manages the server lifecycle)
if (!process.env.CI) {
  (config as any).webServer = {
    command:
      "cargo run -p ferro-server -- --admin-user test --admin-password test --data-dir /tmp/ferro-e2e --storage local:/tmp/ferro-e2e-data",
    url: "http://localhost:8080/.well-known/ferro",
    reuseExistingServer: true,
    timeout: 300_000,
  };
}

export default defineConfig(config);

import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './tests',
  timeout: 30000,
  retries: 0,
  use: {
    baseURL: process.env.FERRO_URL || 'http://localhost:8080',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
  },
  projects: [
    { name: 'chromium', use: { browserName: 'chromium' } },
    { name: 'firefox', use: { browserName: 'firefox' } },
    { name: 'webkit', use: { browserName: 'webkit' } },
  ],
  reporter: [['html', { open: 'never' }]],
  webServer: {
    command: 'cargo run --release --bin ferro-server -- --admin-user e2e-admin --admin-password e2e-test-token --port 18080 --data-dir /tmp/ferro-e2e-test --storage local:/tmp/ferro-e2e-test-data',
    url: 'http://localhost:18080/healthz',
    reuseExistingServer: !process.env.CI,
    timeout: 120000,
  },
});

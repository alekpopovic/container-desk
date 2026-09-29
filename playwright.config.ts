import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests/ui",
  testMatch: "*.spec.ts",
  fullyParallel: true,
  workers: 2,
  retries: 0,
  reporter: "list",
  use: {
    baseURL: "http://127.0.0.1:1431",
    browserName: "chromium",
    screenshot: "only-on-failure",
  },
  projects: [
    {
      name: "wide-light",
      use: { viewport: { width: 1280, height: 800 }, colorScheme: "light" },
    },
    {
      name: "wide-dark",
      use: { viewport: { width: 1280, height: 800 }, colorScheme: "dark" },
    },
    {
      name: "narrow-light",
      use: { viewport: { width: 800, height: 700 }, colorScheme: "light" },
    },
    {
      name: "narrow-dark",
      use: { viewport: { width: 800, height: 700 }, colorScheme: "dark" },
    },
    {
      name: "minimum-light",
      use: { viewport: { width: 640, height: 480 }, colorScheme: "light" },
    },
    {
      name: "minimum-dark",
      use: { viewport: { width: 640, height: 480 }, colorScheme: "dark" },
    },
  ],
  webServer: {
    command: "npm run dev -- --mode browser-test --port 1431",
    url: "http://127.0.0.1:1431",
    reuseExistingServer: false,
    timeout: 30000,
  },
});

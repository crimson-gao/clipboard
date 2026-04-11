import { existsSync } from 'node:fs';
import { defineConfig } from '@playwright/test';

const localChromeExecutable =
  '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';

export default defineConfig({
  testDir: './tests/e2e',
  timeout: 30_000,
  use: {
    baseURL: 'http://127.0.0.1:4173',
    launchOptions: existsSync(localChromeExecutable)
      ? {
          executablePath: localChromeExecutable,
        }
      : undefined,
    trace: 'retain-on-failure',
  },
  webServer: {
    command: 'npm run preview -- --host 127.0.0.1 --port 4173',
    port: 4173,
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});

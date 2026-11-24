import { fileURLToPath } from 'node:url'
import { defineConfig } from '@playwright/test'

const baseURL = 'http://127.0.0.1:4175'

export default defineConfig({
  testDir: '.',
  testMatch: '**/*.pw.ts',
  outputDir: '../../node_modules/.cache/sqlkit-ui-test-results',
  use: {
    baseURL,
    browserName: 'chromium',
    locale: 'en-US',
  },
  webServer: {
    cwd: fileURLToPath(new URL('../../', import.meta.url)),
    command: 'vite preview --host 127.0.0.1 --port 4175 --strictPort',
    url: `${baseURL}/connections`,
  },
})

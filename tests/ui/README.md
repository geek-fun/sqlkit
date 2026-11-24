# SQLKit UI tests

Playwright opens SQLKit's production frontend from `dist` in Chromium. Tests use
the normal routes, stores, and components. The Ubuntu CI job runs these tests
against the build it already produced before packaging the desktop app.

```sh
npx playwright install --only-shell chromium
npm run build
npm run test:ui
```

Use the shared `fixtures/app` test for workflows that call Tauri. It installs
Tauri's official IPC and event mocks before the app starts. Each test gets a
fresh browser context and in-memory native storage. Register synthetic native
responses with `tauri.handle` in the relevant workflow test. Unexpected IPC
commands fail the test.

Assert what the user sees or does. The query test opens a saved connection,
creates a query tab, and checks the rendered completion popup and keyboard
insertion. It exercises Cassandra, PostgreSQL, and MySQL through their real
routing and editor configuration.

Keep completion rules and store logic in the Jest suite. Use Rust and JDBC
integration tests for live database behavior. The Chromium suite covers the
frontend's behavior at the Tauri boundary. Desktop WebView and OS behavior
require native validation.

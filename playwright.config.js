const { defineConfig } = require("@playwright/test");

function requiredPort(name) {
  const port = Number(process.env[name]);
  if (!Number.isSafeInteger(port) || port < 1 || port > 65535) {
    throw new Error(`Use tests/frontend/run.mjs to allocate ${name}`);
  }
  return port;
}

const port = requiredPort("XCZS_FRONTEND_TEST_PORT");
const projectName = process.env.XCZS_FRONTEND_TEST_PROJECT;
if (!["chromium", "firefox", "edge"].includes(projectName)) {
  throw new Error("Use tests/frontend/run.mjs to select browser test projects");
}

const browser = projectName === "edge"
  ? { browserName: "chromium", channel: "msedge" }
  : { browserName: projectName };
const baseURL = `https://127.0.0.1:${port}`;

module.exports = defineConfig({
  testDir: "./tests/frontend",
  testIgnore: "unit/**",
  // The isolated gateway deliberately presents one client address to Xczs.
  // Keep UI cases serial so unrelated test logins cannot contend for the
  // production global/source token buckets and become scheduler-dependent.
  workers: 1,
  retries: 1,
  failOnFlakyTests: true,
  timeout: 30_000,
  expect: {
    timeout: 8_000,
  },
  reporter: [["line"]],
  use: {
    ignoreHTTPSErrors: true,
    viewport: {
      width: 1280,
      height: 800,
    },
    trace: "retain-on-failure",
  },
  projects: [{
    name: projectName,
    use: {
      ...browser,
      baseURL,
    },
  }],
  webServer: {
    command: "node tests/frontend/server.mjs",
    url: `${baseURL}/__xczs__/login`,
    env: {
      XCZS_FRONTEND_TEST_PORT: String(port),
    },
    reuseExistingServer: false,
    timeout: 120_000,
    ignoreHTTPSErrors: true,
  },
});

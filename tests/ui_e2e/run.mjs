import { chromium } from "playwright";
import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const root = process.env.JSONSHEET_ROOT || process.cwd();
const fixture = path.join(root, "tests", "data", "types.json");
const exe = path.join(root, "target", "debug", "jsonsheet.exe");

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function runOrThrow(cmd, args, options = {}) {
  const result = spawnSync(cmd, args, { stdio: "inherit", ...options });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    throw new Error(`${cmd} ${args.join(" ")} failed with code ${result.status}`);
  }
}

async function connectWithRetry(url, attempts = 50) {
  let lastError;
  for (let i = 0; i < attempts; i += 1) {
    try {
      return await chromium.connectOverCDP(url);
    } catch (err) {
      lastError = err;
      await sleep(200);
    }
  }
  throw lastError || new Error("Failed to connect to CDP");
}

async function findAppPage(browser, attempts = 50) {
  for (let i = 0; i < attempts; i += 1) {
    for (const context of browser.contexts()) {
      for (const page of context.pages()) {
        try {
          const title = await page.title();
          if (title.includes("JsonSheet")) {
            return page;
          }
        } catch {
          // Ignore pages that are not ready yet
        }
      }
    }
    await sleep(200);
  }
  throw new Error("JsonSheet page not found via CDP");
}

async function main() {
  runOrThrow("cargo", ["build", "--quiet"], { cwd: root });

  const userDataDir = fs.mkdtempSync(path.join(os.tmpdir(), "jsonsheet-ui-"));
  const env = {
    ...process.env,
    JSONSHEET_OPEN: fixture,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: "--remote-debugging-port=9222",
    WEBVIEW2_USER_DATA_FOLDER: userDataDir,
  };

  const app = spawn(exe, {
    cwd: root,
    env,
    stdio: "inherit",
  });

  let browser;
  try {
    browser = await connectWithRetry("http://127.0.0.1:9222");
    const page = await findAppPage(browser);

    await page.waitForSelector("#table-container", { timeout: 15000 });

    await page.click("#cell-0-name");
    await page.fill("#cell-input-0-name", "Zed");
    await page.keyboard.press("Enter");
    await page.waitForSelector("#cell-0-name:has-text(\"Zed\")");

    await page.click("#btn-add-row");
    const rowCount = await page.locator("tbody tr").count();
    if (rowCount < 3) {
      throw new Error(`Expected at least 3 rows after add, got ${rowCount}`);
    }

    await page.fill("#input-new-column", "department");
    await page.click("#btn-add-column");
    await page.waitForSelector("#col-department", { timeout: 5000 });
  } finally {
    if (browser) {
      await browser.close().catch(() => {});
    }
    if (app) {
      app.kill();
    }
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});

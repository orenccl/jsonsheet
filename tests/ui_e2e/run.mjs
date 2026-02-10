import { chromium } from "playwright";
import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs";
import net from "node:net";
import os from "node:os";
import path from "node:path";

const root = process.env.JSONSHEET_ROOT || process.cwd();
const fixture = path.join(root, "tests", "data", "types.json");

function resolveBinaryPath() {
  const windowsExe = path.join(root, "target", "debug", "jsonsheet.exe");
  const unixExe = path.join(root, "target", "debug", "jsonsheet");

  if (process.platform === "win32") {
    return windowsExe;
  }

  if (fs.existsSync(unixExe)) {
    return unixExe;
  }

  // Fallback for mixed environments where .exe may still be produced.
  return windowsExe;
}

const exe = resolveBinaryPath();

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

function getFreePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      if (!address || typeof address === "string") {
        server.close();
        reject(new Error("Failed to allocate a local port"));
        return;
      }
      const { port } = address;
      server.close((err) => {
        if (err) reject(err);
        else resolve(port);
      });
    });
  });
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

async function listPages(browser) {
  const pages = [];
  for (const context of browser.contexts()) {
    for (const page of context.pages()) {
      let title = "";
      try {
        title = await page.title();
      } catch {
        // ignore page title errors
      }
      pages.push({ url: page.url(), title });
    }
  }
  return pages;
}

async function findAppPage(browser, attempts = 50) {
  for (let i = 0; i < attempts; i += 1) {
    for (const context of browser.contexts()) {
      for (const page of context.pages()) {
        try {
          const url = page.url();
          if (url.startsWith("devtools://")) {
            continue;
          }

          const markerVisible = await page
            .locator(".app, #table-container, #empty-message")
            .first()
            .isVisible({ timeout: 300 })
            .catch(() => false);

          if (markerVisible || url.startsWith("dioxus://") || url.startsWith("http://dioxus.")) {
            return page;
          }
        } catch {
          // Ignore pages that are not ready yet
        }
      }
    }
    await sleep(200);
  }

  const pages = await listPages(browser);
  throw new Error(`JsonSheet page not found via CDP. Pages: ${JSON.stringify(pages)}`);
}

async function waitForRowCount(page, expected, timeout = 5000) {
  await page.waitForFunction(
    (count) => document.querySelectorAll("tbody tr").length === count,
    expected,
    { timeout }
  );
}

async function assertCellContains(page, rowIndex, column, expected, timeout = 5000) {
  const selector = `#cell-${rowIndex}-${column}`;
  await page.waitForFunction(
    ({ selector: s, expectedText }) => {
      const el = document.querySelector(s);
      if (!el) return false;
      return (el.textContent || "").includes(expectedText);
    },
    { selector, expectedText: expected },
    { timeout }
  );
}

async function assertSummaryContains(page, column, expected, timeout = 5000) {
  const selector = `#summary-${column}`;
  await page.waitForFunction(
    ({ selector: s, expectedText }) => {
      const el = document.querySelector(s);
      if (!el) return false;
      return (el.textContent || "").includes(expectedText);
    },
    { selector, expectedText: expected },
    { timeout }
  );
}

async function assertSearchHighlightStyle(page, rowIndex, column, timeout = 5000) {
  const selector = `#cell-${rowIndex}-${column}`;
  await page.waitForSelector(`${selector}.search-match`, { timeout });
  const bg = await page.$eval(selector, (el) => getComputedStyle(el).backgroundColor);
  const nums = String(bg)
    .match(/\d+/g)
    ?.slice(0, 3)
    .map((v) => Number(v));

  const normal = nums?.[0] === 255 && nums?.[1] === 244 && nums?.[2] === 193;
  const hover = nums?.[0] === 255 && nums?.[1] === 232 && nums?.[2] === 156;
  if (!normal && !hover) {
    throw new Error(`Search highlight style not applied. backgroundColor=${bg}`);
  }
}

async function assertElementText(page, selector, expected, timeout = 5000) {
  await page.waitForFunction(
    ({ selector: s, expectedText }) => {
      const el = document.querySelector(s);
      if (!el) return false;
      return (el.textContent || "").trim() === expectedText;
    },
    { selector, expectedText: expected },
    { timeout }
  );
}

async function assertInputPlaceholder(page, selector, expected, timeout = 5000) {
  await page.waitForFunction(
    ({ selector: s, expectedText }) => {
      const el = document.querySelector(s);
      if (!(el instanceof HTMLInputElement)) return false;
      return el.placeholder === expectedText;
    },
    { selector, expectedText: expected },
    { timeout }
  );
}

async function main() {
  runOrThrow("cargo", ["build", "--quiet"], { cwd: root });

  const cdpPort = await getFreePort();
  const tempFixtureDir = fs.mkdtempSync(path.join(os.tmpdir(), "jsonsheet-fixture-"));
  const fixturePath = path.join(tempFixtureDir, "types.json");
  fs.copyFileSync(fixture, fixturePath);
  const fixtureSidecar = `${fixture}.jsheet`;
  if (fs.existsSync(fixtureSidecar)) {
    fs.copyFileSync(fixtureSidecar, `${fixturePath}.jsheet`);
  }

  const userDataDir = fs.mkdtempSync(path.join(os.tmpdir(), "jsonsheet-ui-"));
  const env = {
    ...process.env,
    JSONSHEET_OPEN: fixturePath,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${cdpPort}`,
    WEBVIEW2_USER_DATA_FOLDER: userDataDir,
  };

  const app = spawn(exe, {
    cwd: root,
    env,
    stdio: "inherit",
  });

  let browser;
  try {
    browser = await connectWithRetry(`http://127.0.0.1:${cdpPort}`);
    const page = await findAppPage(browser);

    await page.waitForSelector("#table-container", { timeout: 15000 });
    await waitForRowCount(page, 2, 10000);

    // Phase 5: .jsheet sidecar (computed columns + summaries + type constraints).
    await page.waitForSelector("#col-age2", { timeout: 5000 });
    await assertCellContains(page, 0, "age2", "60");
    await assertSummaryContains(page, "age", "27.5");
    await assertSummaryContains(page, "age2", "110");

    // Cell context menu: range-select then batch update formula.
    await page.click("#cell-0-age2");
    await page.click("#cell-1-age2", { modifiers: ["Shift"] });
    await page.click("#cell-1-age2", { button: "right" });
    await page.fill("#context-formula", "=age * 3");
    await page.click("#btn-context-apply-formula");
    await assertCellContains(page, 0, "age2", "90");
    await assertCellContains(page, 1, "age2", "75");
    await assertSummaryContains(page, "age2", "165");

    // Apply cell style from context menu.
    await page.fill("#context-text-color", "#ff0000");
    await page.fill("#context-bg-color", "#ffffcc");
    await page.click("#btn-context-apply-style");

    // Comment column toggle should be available in header controls.
    await page.fill("#input-new-column", "note");
    await page.click("#btn-add-column");
    await page.waitForSelector("#col-note", { timeout: 5000 });
    await page.click("#meta-comment-note");

    await page.click("#cell-0-age");
    await page.fill("#cell-input-0-age", "oops");
    await page.keyboard.press("Enter");
    await assertCellContains(page, 0, "age", "30");

    // Phase 8: enum validation should expose dropdown options in cell editor.
    await page.fill("#meta-val-enum-name", "Alice, Bob, Charlie, Zed");
    await page.click("#cell-0-name");
    await page.waitForSelector("#cell-input-0-name[list^='enum-options-']", {
      timeout: 5000,
    });
    await page.click("#cell-1-age");
    await page.waitForSelector("#cell-0-name", { timeout: 5000 });

    // Phase 4: i18n language switch.
    await assertElementText(page, "#btn-open", "Open");
    await assertInputPlaceholder(page, "#input-search-query", "Search all cells");
    await page.selectOption("#select-language", "zh-Hant");
    await assertElementText(page, "#btn-open", "開啟");
    await assertInputPlaceholder(page, "#input-search-query", "搜尋所有儲存格");
    await page.selectOption("#select-language", "en");
    await assertElementText(page, "#btn-open", "Open");

    // Phase 2 baseline: edit cell.
    await page.click("#cell-0-name");
    await page.fill("#cell-input-0-name", "Zed");
    await page.keyboard.press("Enter");
    await assertCellContains(page, 0, "name", "Zed");

    // Phase 3: sort + undo/redo.
    await page.click("#sort-age");
    await assertCellContains(page, 0, "name", "Bob");

    await page.click("#btn-undo");
    await assertCellContains(page, 0, "name", "Zed");

    await page.click("#btn-redo");
    await assertCellContains(page, 0, "name", "Bob");

    // Phase 3: filter.
    await page.selectOption("#select-filter-column", "name");
    await page.fill("#input-filter-query", "bo");
    await waitForRowCount(page, 1);
    await assertCellContains(page, 0, "name", "Bob");

    // Phase 3: search highlight.
    await page.fill("#input-search-query", "bo");
    await page.waitForSelector("#cell-0-name.search-match", { timeout: 5000 });
    await assertSearchHighlightStyle(page, 0, "name");

    // Clear filter and verify rows restore while keeping sort/search state.
    await page.click("#btn-clear-filter");
    await waitForRowCount(page, 2);

    // Phase 9: multi-sheet tabs should preserve each sheet state independently.
    await page.click("#btn-new-tab");
    await page.waitForSelector("#tab-1", { timeout: 5000 });
    await page.waitForSelector("#empty-message", { timeout: 5000 });
    await page.click("#tab-0");
    await page.waitForSelector("#table-container", { timeout: 5000 });
    await waitForRowCount(page, 2);

    // Keep existing coverage for row/column edits.
    await page.click("#btn-add-row");
    await page.waitForTimeout(200);
    let rowCount = await page.locator("tbody tr").count();
    if (rowCount < 3) {
      await page.evaluate(() => {
        const button = document.getElementById("btn-add-row");
        if (button instanceof HTMLElement) {
          button.click();
        }
      });
      await page.waitForFunction(
        () => document.querySelectorAll("tbody tr").length >= 3,
        null,
        { timeout: 5000 }
      );
      rowCount = await page.locator("tbody tr").count();
    }
    if (rowCount < 3) {
      throw new Error(`Expected at least 3 rows after add, got ${rowCount}`);
    }

    await page.fill("#input-new-column", "department");
    await page.click("#btn-add-column");
    await page.waitForSelector("#col-department", { timeout: 5000 });

    // Phase 9: auto-fill drag handle (single numeric source should increment).
    await page.click("#cell-0-age");
    await page.fill("#cell-input-0-age", "10");
    await page.keyboard.press("Enter");
    await assertCellContains(page, 0, "age", "10");
    await page.dragAndDrop("#cell-0-age .fill-handle", "#cell-2-age");
    await assertCellContains(page, 1, "age", "11");
    await assertCellContains(page, 2, "age", "12");
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

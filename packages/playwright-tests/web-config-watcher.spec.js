// @ts-check
const { test, expect } = require("@playwright/test");
const fs = require("fs");

// Full rebuilds on Cargo.toml/Dioxus.toml edits go through the fat-rebuild path
// rather than hotpatch, so they take roughly as long as a cold build.
const rebuildTimeout = { timeout: 1000 * 60 * 2 };

const dioxusTomlPath = "web-config-watcher-temp/Dioxus.toml";
const cargoTomlPath = "web-config-watcher-temp/Cargo.toml";

const initialDioxusToml = `[application]
name = "config-watcher-fixture"

[web.app]
title = "initial-title"
`;

const initialCargoToml = `[package]
name = "dioxus-playwright-config-watcher-test"
version = "0.0.1"
edition = "2024"
description = "Playwright fixture for Cargo.toml / Dioxus.toml live-edit detection"
license = "MIT OR Apache-2.0"
publish = false

[dependencies]
dioxus = { path = "../../dioxus", features = ["web"] }

[workspace]
`;

test("config-watcher classifies edits", async ({ page }) => {
  // Reset fixture state to known-good baseline so re-runs are deterministic.
  fs.writeFileSync(dioxusTomlPath, initialDioxusToml);
  fs.writeFileSync(cargoTomlPath, initialCargoToml);

  await page.goto("http://localhost:9982");

  // ---- 1. Initial state: title comes from Dioxus.toml, counter increments ----
  await expect(page).toHaveTitle("initial-title");
  const main = page.locator("#main");
  await expect(main).toContainText("Count: 0");
  const button = page.locator("button#increment-button");
  await button.click();
  await expect(main).toContainText("Count: 1");

  // ---- 2. Rebuild path: change [web.app].title triggers full rebuild ----
  // The title is baked into the served HTML at build time, so a successful
  // rebuild + page reload swaps document.title.
  fs.writeFileSync(
    dioxusTomlPath,
    initialDioxusToml.replace("initial-title", "rebuilt-title")
  );
  await expect(page).toHaveTitle("rebuilt-title", rebuildTimeout);

  // After the page reloads the counter resets — confirms the page actually navigated,
  // not just had its title patched.
  await expect(main).toContainText("Count: 0");
  await button.click();
  await expect(main).toContainText("Count: 1");

  // ---- 3. WarnRestart path: adding [[web.proxy]] should NOT trigger a rebuild ----
  // Proxy config is read at devserver boot only. We expect a warning in the CLI log
  // and zero impact on the running app: the counter and title both survive.
  fs.writeFileSync(
    dioxusTomlPath,
    initialDioxusToml.replace("initial-title", "rebuilt-title") +
      "\n[[web.proxy]]\nbackend = \"http://example.invalid/api\"\n"
  );

  // Give the watcher plenty of time to observe the change and (if buggy) kick off a rebuild.
  await page.waitForTimeout(8000);

  // Counter must still be 1 — a rebuild would have reloaded and reset it to 0.
  await expect(main).toContainText("Count: 1");
  await expect(page).toHaveTitle("rebuilt-title");

  // ---- 4. Ignore path: editing [package].description should NOT trigger anything ----
  fs.writeFileSync(
    cargoTomlPath,
    initialCargoToml.replace(
      "Playwright fixture for Cargo.toml / Dioxus.toml live-edit detection",
      "an entirely cosmetic edit that should be ignored"
    )
  );

  await page.waitForTimeout(8000);
  await expect(main).toContainText("Count: 1");

  // ---- 5. Rebuild path: adding a new feature triggers full rebuild ----
  // We use [features] instead of [dependencies] so we don't have to download anything.
  fs.writeFileSync(
    cargoTomlPath,
    initialCargoToml.replace(
      "[workspace]",
      "[features]\nbar = []\n\n[workspace]"
    )
  );

  // After rebuild the counter resets again. Use a long timeout — this is a fat rebuild.
  await expect(main).toContainText("Count: 0", rebuildTimeout);

  // Reset fixture back to baseline so a re-run starts from a clean slate.
  fs.writeFileSync(dioxusTomlPath, initialDioxusToml);
  fs.writeFileSync(cargoTomlPath, initialCargoToml);
});

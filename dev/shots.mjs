#!/usr/bin/env node
// Design review screenshots of the running dev stack (mock upstreams + backend +
// Vite): login and dashboard for every theme, light and dark, at desktop and
// phone widths.
//
//   node dev/shots.mjs [out dir] [base url] [themes, e.g. paper,panel]
import { mkdirSync } from "node:fs";
import { createRequire } from "node:module";

const require = createRequire(new URL("../web/package.json", import.meta.url));
const { chromium } = require("playwright");

const out = process.argv[2] ?? "dev/shots";
const base = process.argv[3] ?? "http://localhost:5173";
const themes = (process.argv[4] ?? "paper,panel,swiss").split(",");
mkdirSync(out, { recursive: true });

const views = [
  { name: "desktop", width: 1440, height: 900 },
  { name: "phone", width: 390, height: 844, mobile: true },
];

const browser = await chromium.launch();
for (const theme of themes) {
  for (const scheme of ["light", "dark"]) {
    for (const view of views) {
      const context = await browser.newContext({
        viewport: { width: view.width, height: view.height },
        deviceScaleFactor: view.mobile ? 2 : 1,
        isMobile: !!view.mobile,
        hasTouch: !!view.mobile,
        colorScheme: scheme,
        reducedMotion: "reduce",
      });
      // The theme choice lives in localStorage; set it before the app boots.
      await context.addInitScript((name) => localStorage.setItem("hp-theme", name), theme);
      const page = await context.newPage();
      await page.goto(base, { waitUntil: "networkidle" });
      await page.waitForTimeout(600);
      await page.screenshot({ path: `${out}/${theme}-${scheme}-${view.name}-login.png` });
      const response = await page.request.post(`${base}/api/v1/auth/login`, {
        headers: { origin: base },
        data: { username: "chao", password: "homepage-dev-password" },
      });
      if (!response.ok()) throw new Error(`login failed: ${response.status()}`);
      await page.goto(base, { waitUntil: "networkidle" });
      await page.waitForTimeout(1200);
      await page.screenshot({ path: `${out}/${theme}-${scheme}-${view.name}.png`, fullPage: true });
      if (view.name === "desktop") await page.screenshot({ path: `${out}/${theme}-${scheme}-${view.name}-fold.png` });
      await context.close();
    }
  }
}
await browser.close();
console.log(`screenshots in ${out}`);

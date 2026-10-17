// Screenshot the settings dialog for one theme: node dev/shot-dialog.mjs <theme> <scheme> <out.png>
import { createRequire } from "node:module";
const require = createRequire(new URL("../web/package.json", import.meta.url));
const { chromium } = require("playwright");
const [theme = "paper", scheme = "light", out = "dev/shots/dialog.png"] = process.argv.slice(2);
const base = "http://localhost:5173";
const browser = await chromium.launch();
const context = await browser.newContext({ viewport: { width: 1200, height: 900 }, colorScheme: scheme, reducedMotion: "reduce" });
await context.addInitScript((name) => localStorage.setItem("hp-theme", name), theme);
const page = await context.newPage();
await page.request.post(`${base}/api/v1/auth/login`, { headers: { origin: base }, data: { username: "chao", password: "homepage-dev-password" } });
await page.goto(base, { waitUntil: "networkidle" });
await page.getByRole("button", { name: /设置|v\d/ }).first().click();
await page.waitForTimeout(500);
await page.screenshot({ path: out });
await browser.close();
console.log(out);

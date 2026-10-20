// Renders the PNG icons from web/public/favicon.svg: node dev/icons.mjs
// Safari and iOS do not take SVG favicons, so the PNGs are what they show.
import { createRequire } from "node:module";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
const require = createRequire(new URL("../web/package.json", import.meta.url));
const { chromium } = require("playwright");

const svg = readFileSync(new URL("../web/public/favicon.svg", import.meta.url), "utf8");
const sizes = [
  ["favicon-16.png", 16],
  ["favicon-32.png", 32],
  ["apple-touch-icon.png", 180],
  ["icon-192.png", 192],
  ["icon-512.png", 512],
];
const browser = await chromium.launch();
const context = await browser.newContext({ colorScheme: "light", deviceScaleFactor: 1 });
const page = await context.newPage();
for (const [name, size] of sizes) {
  await page.setViewportSize({ width: size, height: size });
  // The touch icon is squared off: iOS rounds it itself, and a rounded PNG shows a white rim.
  const squared = name === "apple-touch-icon.png" ? svg.replace('rx="14"', 'rx="0"') : svg;
  await page.setContent(`<body style="margin:0;background:transparent">${squared.replace("<svg ", `<svg width="${size}" height="${size}" `)}</body>`);
  await page.screenshot({ path: fileURLToPath(new URL(`../web/public/${name}`, import.meta.url)), omitBackground: true });
  console.log(name, size);
}
await browser.close();

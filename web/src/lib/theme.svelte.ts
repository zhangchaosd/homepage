/** Look (theme) and appearance (light/dark) for this browser.
 *
 * The server supplies a default theme; the viewer's choice is kept in
 * localStorage. Both are applied as `data-theme` / `data-mode` on <html>, which
 * app.css keys all tokens off. `data-family` names the structure a theme is
 * built on (ruled, panel or swiss), so components style a family once and the
 * newer themes (themes/*.css) only add their own look on top. */

export type ThemeName = "paper" | "panel" | "swiss" | "scifi" | "future" | "tech" | "industrial" | "native";
export type Mode = "auto" | "light" | "dark";

export const THEMES: { id: ThemeName; name: string; blurb: string }[] = [
  { id: "paper", name: "晨报", blurb: "衬线字、细线栏目、纸色与红墨" },
  { id: "panel", name: "控制台", blurb: "等宽读数、指示灯、琥珀色面板" },
  { id: "swiss", name: "工作台", blurb: "粗黑标题、硬边框、色块与宝蓝" },
  { id: "scifi", name: "科幻", blurb: "舰桥 HUD、角标边框、青色读数" },
  { id: "future", name: "未来", blurb: "陶瓷与光、大圆角、紫色与荧光绿" },
  { id: "tech", name: "科技", blurb: "遥测仪表、电光蓝、延迟读数" },
  { id: "industrial", name: "工业", blurb: "服务器面板、点阵液晶、实体按键" },
  { id: "native", name: "原生", blurb: "系统字体、分组列表、毛玻璃工具栏" },
];

const FAMILY: Record<ThemeName, "ruled" | "panel" | "swiss"> = {
  paper: "ruled",
  panel: "panel",
  swiss: "swiss",
  scifi: "panel",
  future: "panel",
  tech: "panel",
  industrial: "panel",
  native: "panel",
};

const THEME_KEY = "hp-theme";
const MODE_KEY = "hp-mode";

function read(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function write(key: string, value: string | null) {
  try {
    if (value === null) localStorage.removeItem(key);
    else localStorage.setItem(key, value);
  } catch {
    // Private windows: the choice simply does not persist.
  }
}

const isTheme = (value: unknown): value is ThemeName => THEMES.some((item) => item.id === value);
const isMode = (value: unknown): value is Mode => value === "auto" || value === "light" || value === "dark";

export const theme = $state<{ name: ThemeName; mode: Mode; serverDefault: ThemeName; overridden: boolean }>({
  name: "paper",
  mode: "auto",
  serverDefault: "paper",
  overridden: false,
});

function apply() {
  const root = document.documentElement;
  root.dataset.theme = theme.name;
  root.dataset.family = FAMILY[theme.name];
  root.dataset.mode = theme.mode;
  // Tokens use light-dark(), so the root's color-scheme decides the palette.
  root.style.colorScheme = theme.mode === "auto" ? "light dark" : theme.mode;
  // Keep the browser chrome in step with the page background.
  requestAnimationFrame(() => {
    const bg = getComputedStyle(document.body).backgroundColor;
    for (const meta of document.querySelectorAll<HTMLMetaElement>('meta[name="theme-color"]')) meta.content = bg;
  });
}

/** Called once with the server's default; a stored choice wins over it. */
export function initTheme(serverDefault: string) {
  theme.serverDefault = isTheme(serverDefault) ? serverDefault : "paper";
  const stored = read(THEME_KEY);
  theme.overridden = isTheme(stored);
  theme.name = isTheme(stored) ? stored : theme.serverDefault;
  const mode = read(MODE_KEY);
  theme.mode = isMode(mode) ? mode : "auto";
  apply();
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", apply);
}

export function setTheme(name: ThemeName) {
  theme.name = name;
  theme.overridden = name !== theme.serverDefault;
  write(THEME_KEY, theme.overridden ? name : null);
  apply();
}

export function setMode(mode: Mode) {
  theme.mode = mode;
  write(MODE_KEY, mode === "auto" ? null : mode);
  apply();
}

<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "./lib/Icon.svelte";
  import { ApiError, checkUpdate, getSystem, installUpdate, saveUpdateSettings, type UpdateSettings, type UpdateStatus } from "./lib/api";
  import { ago } from "./lib/time";
  import { now } from "./lib/clock.svelte";
  import { THEMES, setMode, setTheme, theme, type Mode } from "./lib/theme.svelte";

  let { status, onstatus, onclose }: { status: UpdateStatus | null; onstatus: (status: UpdateStatus) => void; onclose: () => void } = $props();

  let dialog: HTMLDialogElement | undefined = $state();
  let phase = $state<"" | "checking" | "installing" | "restarting">("");
  let message = $state("");

  const weekdays = ["周一", "周二", "周三", "周四", "周五", "周六", "周日"];
  const modes: { id: Mode; name: string }[] = [
    { id: "auto", name: "跟随系统" },
    { id: "light", name: "浅色" },
    { id: "dark", name: "深色" },
  ];
  const published = new Intl.DateTimeFormat("zh-CN", { year: "numeric", month: "long", day: "numeric" });
  const busy = $derived(phase === "installing" || phase === "restarting");

  onMount(() => dialog?.showModal());

  function close() {
    if (busy) return;
    dialog?.close();
  }

  function explain(cause: unknown) {
    return cause instanceof ApiError || cause instanceof Error ? cause.message : "操作失败";
  }

  async function check() {
    if (phase) return;
    phase = "checking";
    message = "";
    try {
      onstatus(await checkUpdate());
    } catch (cause) {
      message = explain(cause);
    } finally {
      phase = "";
    }
  }

  async function save(patch: Partial<UpdateSettings>) {
    if (!status) return;
    message = "";
    try {
      onstatus(await saveUpdateSettings({ ...status.settings, ...patch }));
    } catch (cause) {
      message = explain(cause);
    }
  }

  async function install() {
    const version = status?.latest?.version;
    if (!version || phase) return;
    phase = "installing";
    message = "";
    try {
      const result = await installUpdate(version);
      if (!result.restarting) {
        phase = "";
        message = `已安装 v${version}，请手动重启服务。`;
        return;
      }
      phase = "restarting";
      await waitForVersion(version);
      location.reload();
    } catch (cause) {
      phase = "";
      message = explain(cause);
    }
  }

  /** The server re-executes itself; poll until the new version answers. */
  async function waitForVersion(version: string) {
    const deadline = Date.now() + 90_000;
    await new Promise((resolve) => setTimeout(resolve, 1500));
    while (Date.now() < deadline) {
      try {
        if ((await getSystem()).version === version) return;
      } catch {
        // Not listening yet.
      }
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }
    throw new Error("新版本没有按时启动，请检查服务日志。");
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<dialog
  bind:this={dialog}
  onclose={onclose}
  oncancel={(event) => {
    if (busy) event.preventDefault();
  }}
  onclick={(event) => {
    if (event.target === dialog) close();
  }}
  aria-labelledby="settings-title"
>
  <div class="sheet">
    <header>
      <h2 id="settings-title">设置</h2>
      <button class="close pressable" onclick={close} aria-label="关闭" disabled={busy}><Icon name="x" size={16} /></button>
    </header>

    <section>
      <h3 class="label">外观</h3>
      <div class="themes" role="radiogroup" aria-label="主题">
        {#each THEMES as item (item.id)}
          <button
            class="theme pressable"
            class:on={theme.name === item.id}
            role="radio"
            aria-checked={theme.name === item.id}
            onclick={() => setTheme(item.id)}
          >
            <span class="swatch" data-swatch={item.id} aria-hidden="true"><i></i><i></i><i></i></span>
            <span class="name">{item.name}</span>
            <span class="blurb">{item.blurb}</span>
          </button>
        {/each}
      </div>
      <div class="segmented" role="radiogroup" aria-label="明暗">
        {#each modes as item (item.id)}
          <button class:on={theme.mode === item.id} role="radio" aria-checked={theme.mode === item.id} onclick={() => setMode(item.id)}>{item.name}</button>
        {/each}
      </div>
      <p class="hint">
        {theme.overridden ? `这台浏览器的选择会覆盖服务器默认（${THEMES.find((t) => t.id === theme.serverDefault)?.name}）。` : "服务器默认可在 config.toml 的 [profile] theme 中设置。"}
      </p>
    </section>

    <section>
      <h3 class="label">版本与更新</h3>
      {#if !status}
        <p class="hint">无法读取更新状态。</p>
      {:else}
        <div class="versions">
          <div>
            <span class="label">当前版本</span>
            <span class="value mono">v{status.current}</span>
          </div>
          {#if status.available && status.latest}
            <span class="arrow" aria-hidden="true"><Icon name="chevron" size={14} /></span>
            <div>
              <span class="label">最新版本{status.latest.prerelease ? " · 预发布" : ""}</span>
              <span class="value mono accent">v{status.latest.version}</span>
            </div>
          {/if}
        </div>

        {#if status.available && status.latest}
          <div class="release">
            <div class="release-head">
              <span>{status.latest.publishedAt ? published.format(new Date(status.latest.publishedAt)) : ""}</span>
              {#if status.latest.url}
                <a class="link" href={status.latest.url} target="_blank" rel="noopener noreferrer">在 GitHub 查看</a>
              {/if}
            </div>
            {#if status.latest.notes.trim()}
              <pre class="notes">{status.latest.notes.trim()}</pre>
            {/if}
          </div>
        {:else}
          <p class="uptodate"><Icon name="check" size={14} stroke={2.2} />{status.checkedAt ? "已是最新版本" : "还没有检查过更新"}</p>
        {/if}

        <p class="meta">
          {#if status.checkedAt}上次检查 {ago(status.checkedAt, now.value)}{/if}
          {#if status.error}<span class="error">· {status.error}</span>{/if}
        </p>

        <div class="actions">
          <button class="secondary pressable" onclick={check} disabled={!!phase}>
            <Icon name="refresh" size={14} />{phase === "checking" ? "正在检查…" : "检查更新"}
          </button>
          {#if status.available && status.latest}
            <button class="primary pressable" onclick={install} disabled={!!phase || !status.installable}>
              <Icon name="download" size={14} />
              {phase === "installing" ? "正在下载并校验…" : phase === "restarting" ? "正在重启…" : "安装并重启"}
            </button>
          {/if}
        </div>
        {#if status.available && !status.installable}
          <p class="hint">这个平台（{status.platform ?? "未知"}）不支持自动安装，请从 GitHub 手动下载。</p>
        {/if}

        <fieldset disabled={!!phase}>
          <legend class="label">自动检查</legend>
          <div class="row">
            <label>
              <span>频率</span>
              <select value={status.settings.schedule} onchange={(event) => save({ schedule: event.currentTarget.value as UpdateSettings["schedule"] })}>
                <option value="off">关闭</option>
                <option value="daily">每天</option>
                <option value="weekly">每周</option>
              </select>
            </label>
            {#if status.settings.schedule === "weekly"}
              <label>
                <span>星期</span>
                <select value={String(status.settings.weekday)} onchange={(event) => save({ weekday: Number(event.currentTarget.value) })}>
                  {#each weekdays as day, i (day)}<option value={String(i + 1)}>{day}</option>{/each}
                </select>
              </label>
            {/if}
            {#if status.settings.schedule !== "off"}
              <label>
                <span>时间</span>
                <input type="time" value={status.settings.time} onchange={(event) => save({ time: event.currentTarget.value })} />
              </label>
            {/if}
            <label>
              <span>通道</span>
              <select value={status.settings.channel} onchange={(event) => save({ channel: event.currentTarget.value as UpdateSettings["channel"] })}>
                <option value="stable">正式版</option>
                <option value="prerelease">包含预发布</option>
              </select>
            </label>
          </div>
          <p class="hint">自动检查只提示有新版本，安装始终需要手动确认。</p>
        </fieldset>
      {/if}
      <p class="message" role="alert" aria-live="polite">{message}</p>
    </section>
  </div>
</dialog>

<style>
  dialog {
    width: min(calc(100% - 32px), 560px);
    max-height: calc(100dvh - 48px);
    padding: 0;
    border: var(--card-border-w, 1px) solid var(--line);
    border-radius: var(--card-radius);
    background: var(--surface);
    color: var(--ink);
    box-shadow: 0 24px 60px -20px rgba(0, 0, 0, 0.4);
    overflow: auto;
    opacity: 1;
    transform: none;
    transition:
      opacity 200ms var(--ease-out),
      transform 200ms var(--ease-out),
      overlay 200ms allow-discrete,
      display 200ms allow-discrete;
  }
  :global(:root[data-theme="paper"]) dialog,
  :global(:root:not([data-theme])) dialog {
    border-width: 1px;
    background: var(--bg);
  }
  @starting-style {
    dialog[open] {
      opacity: 0;
      transform: scale(0.98) translateY(6px);
    }
  }
  dialog::backdrop {
    background: rgba(10, 10, 9, 0.4);
    backdrop-filter: blur(3px);
  }

  .sheet {
    padding: 20px 24px 22px;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
  }
  h2 {
    margin: 0;
    font-family: var(--font-heading);
    font-weight: var(--w-heading);
    font-size: 22px;
    letter-spacing: var(--ls-heading);
  }
  :global(:root[data-family="panel"]) h2 {
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.14em;
  }
  .close {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    color: var(--muted);
  }
  section {
    padding: 18px 0 6px;
    border-top: 1px solid var(--hair);
  }
  section:first-of-type {
    border-top: 0;
    padding-top: 8px;
  }
  h3 {
    margin: 0 0 12px;
  }

  .themes {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 8px;
  }
  .theme {
    display: grid;
    gap: 8px;
    padding: 12px;
    text-align: left;
    border: 1px solid var(--hair);
    border-radius: var(--tile-radius);
    background: var(--surface-2);
  }
  .theme.on {
    border-color: var(--ink);
    box-shadow: inset 0 0 0 1px var(--ink);
  }
  .theme .name {
    font-weight: 600;
    font-size: 13.5px;
  }
  .theme .blurb {
    font-size: 11.5px;
    color: var(--muted);
    line-height: 1.4;
  }
  .swatch {
    display: flex;
    gap: 3px;
    height: 24px;
  }
  .swatch i {
    flex: 1;
    border-radius: 2px;
  }
  .swatch[data-swatch="paper"] i:nth-child(1) { background: #f3f1ea; border: 1px solid rgba(22, 21, 15, 0.2); }
  .swatch[data-swatch="paper"] i:nth-child(2) { background: #16150f; }
  .swatch[data-swatch="paper"] i:nth-child(3) { background: #c8411e; }
  .swatch[data-swatch="panel"] i:nth-child(1) { background: #0a0b0b; border: 1px solid #2a302f; }
  .swatch[data-swatch="panel"] i:nth-child(2) { background: #161919; border: 1px solid #2a302f; }
  .swatch[data-swatch="panel"] i:nth-child(3) { background: #f5a524; }
  .swatch[data-swatch="scifi"] i:nth-child(1) { background: #04070c; border: 1px solid #1c4652; }
  .swatch[data-swatch="scifi"] i:nth-child(2) { background: #08111a; border: 1px solid #3ee6ff; }
  .swatch[data-swatch="scifi"] i:nth-child(3) { background: #3ee6ff; }
  .swatch[data-swatch="future"] i:nth-child(1) { background: #f1f0f5; border: 1px solid #ffffff; border-radius: 6px; }
  .swatch[data-swatch="future"] i:nth-child(2) { background: #5b3dff; border-radius: 6px; }
  .swatch[data-swatch="future"] i:nth-child(3) { background: #d4ff3a; border-radius: 6px; }
  .swatch[data-swatch="swiss"] i:nth-child(1) { background: #f6f6f3; border: 1.5px solid #0d0d0c; }
  .swatch[data-swatch="swiss"] i:nth-child(2) { background: #0d0d0c; }
  .swatch[data-swatch="swiss"] i:nth-child(3) { background: #1f3fd6; }

  .segmented {
    display: inline-flex;
    margin-top: 12px;
    border: 1px solid var(--hair);
    border-radius: var(--btn-radius);
    overflow: hidden;
  }
  .segmented button {
    padding: 7px 14px;
    font-size: 13px;
    color: var(--muted);
    border-left: 1px solid var(--hair);
    transition: background-color 140ms ease, color 140ms ease;
  }
  .segmented button:first-child {
    border-left: 0;
  }
  .segmented button.on {
    background: var(--ink);
    color: var(--bg);
  }

  .versions {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 12px 14px;
    border: 1px solid var(--hair);
    border-radius: var(--tile-radius);
    background: var(--surface-2);
  }
  .versions > div {
    display: grid;
    gap: 4px;
  }
  .value {
    font-size: 20px;
    font-weight: 600;
  }
  .value.accent {
    color: var(--accent-ink);
  }
  .arrow {
    color: var(--faint);
  }
  .release {
    margin-top: 12px;
  }
  .release-head {
    display: flex;
    justify-content: space-between;
    font-size: 12.5px;
    color: var(--muted);
  }
  .release-head a {
    color: var(--accent-ink);
  }
  .notes {
    margin: 8px 0 0;
    max-height: 200px;
    overflow: auto;
    padding: 12px 14px;
    border: 1px solid var(--hair);
    border-radius: var(--tile-radius);
    background: var(--surface-2);
    font-family: var(--font-sans);
    font-size: 12.5px;
    line-height: 1.6;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    color: var(--ink-2);
  }
  .uptodate {
    display: flex;
    align-items: center;
    gap: 7px;
    margin: 12px 0 0;
    color: var(--ok);
    font-weight: 550;
  }
  .meta {
    min-height: 18px;
    margin: 8px 0 0;
    font-size: 12px;
    color: var(--muted);
  }
  .error {
    color: var(--danger);
  }
  .actions {
    display: flex;
    gap: 8px;
    margin-top: 12px;
  }
  .primary,
  .secondary {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    height: 38px;
    padding: 0 16px;
    font-weight: 550;
    font-size: 13.5px;
    border-radius: var(--btn-radius);
  }
  .primary {
    flex: 1;
    background: var(--btn-bg);
    color: var(--btn-fg);
  }
  .secondary {
    border: 1px solid var(--line);
  }
  .primary:disabled,
  .secondary:disabled {
    opacity: 0.5;
    cursor: default;
    transform: none;
  }
  fieldset {
    margin: 16px 0 0;
    padding: 0;
    border: 0;
  }
  legend {
    padding: 0;
    margin-bottom: 8px;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }
  .row label {
    display: grid;
    gap: 5px;
    flex: 1 1 100px;
  }
  .row label span {
    font-size: 12px;
    color: var(--ink-2);
  }
  select,
  input {
    height: 36px;
    padding: 0 10px;
    border: 1px solid var(--input-border);
    border-radius: var(--input-radius);
    background: var(--input-bg);
    font-size: 14px;
    outline: none;
  }
  select:focus,
  input:focus {
    border-color: var(--accent);
  }
  .hint {
    margin: 10px 0 0;
    font-size: 12px;
    color: var(--muted);
  }
  .message {
    min-height: 18px;
    margin: 10px 0 0;
    font-size: 12.5px;
    color: var(--danger);
  }
  @media (max-width: 480px) {
    .themes {
      grid-template-columns: 1fr;
    }
  }
</style>

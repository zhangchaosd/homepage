<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "./lib/Icon.svelte";
  import {
    ApiError,
    checkUpdate,
    getSystem,
    installUpdate,
    saveUpdateSettings,
    type UpdateSettings,
    type UpdateStatus,
  } from "./lib/api";
  import { ago } from "./lib/time";
  import { now } from "./lib/clock.svelte";

  let { status, onstatus, onclose }: { status: UpdateStatus; onstatus: (status: UpdateStatus) => void; onclose: () => void } = $props();

  let dialog: HTMLDialogElement | undefined = $state();
  let phase = $state<"" | "checking" | "installing" | "restarting">("");
  let message = $state("");

  const weekdays = ["周一", "周二", "周三", "周四", "周五", "周六", "周日"];
  const published = new Intl.DateTimeFormat("zh-CN", { year: "numeric", month: "long", day: "numeric" });

  onMount(() => dialog?.showModal());

  function close() {
    if (phase === "installing" || phase === "restarting") return;
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
    message = "";
    try {
      onstatus(await saveUpdateSettings({ ...status.settings, ...patch }));
    } catch (cause) {
      message = explain(cause);
    }
  }

  async function install() {
    const version = status.latest?.version;
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
    throw new Error("新版本没有按时启动，请检查服务日志（journalctl -u homepage）。");
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<dialog
  bind:this={dialog}
  onclose={onclose}
  oncancel={(event) => {
    if (phase === "installing" || phase === "restarting") event.preventDefault();
  }}
  onclick={(event) => {
    if (event.target === dialog) close();
  }}
  aria-labelledby="update-title"
>
  <div class="sheet">
    <header>
      <h2 id="update-title">版本与更新</h2>
      <button class="close pressable" onclick={close} aria-label="关闭" disabled={phase === "installing" || phase === "restarting"}>
        <Icon name="x" size={16} />
      </button>
    </header>

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
      <section class="release">
        <div class="release-head">
          <span>{status.latest.publishedAt ? published.format(new Date(status.latest.publishedAt)) : ""}</span>
          {#if status.latest.url}
            <a href={status.latest.url} target="_blank" rel="noopener noreferrer">在 GitHub 查看 <Icon name="arrow-up-right" size={12} /></a>
          {/if}
        </div>
        {#if status.latest.notes.trim()}
          <pre class="notes">{status.latest.notes.trim()}</pre>
        {/if}
      </section>
    {:else}
      <p class="uptodate">
        <Icon name="check" size={15} stroke={2.2} />
        {status.checkedAt ? "已是最新版本" : "还没有检查过更新"}
      </p>
    {/if}

    <p class="meta">
      {#if status.checkedAt}上次检查 {ago(status.checkedAt, now.value)}{/if}
      {#if status.error}<span class="error">· {status.error}</span>{/if}
    </p>

    <div class="actions">
      <button class="secondary pressable" onclick={check} disabled={!!phase}>
        <Icon name="refresh" size={14} />
        {phase === "checking" ? "正在检查…" : "检查更新"}
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
      <legend>自动检查</legend>
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
              {#each weekdays as day, index (day)}<option value={String(index + 1)}>{day}</option>{/each}
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

    <p class="message" role="alert" aria-live="polite">{message}</p>
  </div>
</dialog>

<style>
  dialog {
    width: min(calc(100% - 32px), 520px);
    max-height: calc(100dvh - 48px);
    padding: 0;
    border: 1px solid var(--line-strong);
    border-radius: 22px;
    background: var(--surface);
    color: var(--ink);
    box-shadow: var(--shadow-pop);
    overflow: auto;
    opacity: 1;
    transform: none;
    transition:
      opacity 200ms var(--ease-out),
      transform 200ms var(--ease-out),
      overlay 200ms allow-discrete,
      display 200ms allow-discrete;
  }
  @starting-style {
    dialog[open] {
      opacity: 0;
      transform: scale(0.97) translateY(4px);
    }
  }
  dialog::backdrop {
    background: rgba(14, 14, 13, 0.32);
    backdrop-filter: blur(3px);
    transition:
      background-color 200ms ease,
      overlay 200ms allow-discrete,
      display 200ms allow-discrete;
  }
  @starting-style {
    dialog[open]::backdrop {
      background: rgba(14, 14, 13, 0);
    }
  }

  .sheet {
    padding: 20px 22px 18px;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
  }
  h2 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .close {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: 9px;
    color: var(--muted);
  }
  @media (hover: hover) and (pointer: fine) {
    .close:hover {
      background: var(--surface-hover);
      color: var(--ink);
    }
  }

  .versions {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 14px 16px;
    border-radius: 14px;
    background: var(--surface-2);
    border: 1px solid var(--line);
  }
  .versions > div {
    display: grid;
    gap: 2px;
  }
  .label {
    font-size: 12px;
    color: var(--muted);
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
    margin-top: 14px;
  }
  .release-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 12.5px;
    color: var(--muted);
  }
  .release-head a {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    color: var(--accent-ink);
  }
  .notes {
    margin: 8px 0 0;
    max-height: 200px;
    overflow: auto;
    padding: 12px 14px;
    border-radius: 12px;
    background: var(--surface-2);
    border: 1px solid var(--line);
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
    margin: 14px 0 0;
    color: var(--accent-ink);
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
    margin-top: 14px;
  }
  .primary,
  .secondary {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    height: 38px;
    padding: 0 16px;
    border-radius: 11px;
    font-weight: 550;
    font-size: 13.5px;
  }
  .primary {
    flex: 1;
    background: var(--ink);
    color: var(--surface);
  }
  .secondary {
    background: var(--surface-2);
    border: 1px solid var(--line-strong);
  }
  .primary:disabled,
  .secondary:disabled {
    opacity: 0.55;
    cursor: default;
    transform: none;
  }

  fieldset {
    margin: 18px 0 0;
    padding: 16px 0 0;
    border: 0;
    border-top: 1px solid var(--line);
  }
  legend {
    padding: 0;
    font-size: 12px;
    font-weight: 550;
    color: var(--muted);
    letter-spacing: 0.02em;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    margin-top: 10px;
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
    border-radius: 10px;
    border: 1px solid var(--line-strong);
    background: var(--surface-2);
    font-size: 14px;
    outline: none;
  }
  select:focus,
  input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
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
</style>

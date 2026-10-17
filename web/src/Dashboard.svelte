<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "./lib/Icon.svelte";
  import Card from "./cards/Card.svelte";
  import Services from "./cards/Services.svelte";
  import Todos from "./cards/Todos.svelte";
  import Devices from "./cards/Devices.svelte";
  import Commits from "./cards/Commits.svelte";
  import Github from "./cards/Github.svelte";
  import Bookmarks from "./cards/Bookmarks.svelte";
  import Home from "./cards/Home.svelte";
  import Inbox from "./cards/Inbox.svelte";
  import SettingsDialog from "./SettingsDialog.svelte";
  import { ApiError, getDashboard, getUpdate, logout, type Dashboard, type UpdateStatus } from "./lib/api";
  import { dayDiff, greeting } from "./lib/time";
  import { now, startClock } from "./lib/clock.svelte";

  let { initial, onsignedout }: { initial: Dashboard; onsignedout: () => void } = $props();

  // svelte-ignore state_referenced_locally
  let dash = $state<Dashboard>(initial);
  let query = $state("");
  let search: HTMLInputElement | undefined = $state();
  let update = $state<UpdateStatus | null>(null);
  let settingsOpen = $state(false);

  const POLL_MS = 30_000;

  async function reload() {
    try {
      dash = await getDashboard();
    } catch (cause) {
      if (cause instanceof ApiError && cause.status === 401) onsignedout();
    }
  }

  async function signOut() {
    await logout();
    onsignedout();
  }

  onMount(() => {
    startClock();
    // Updates are optional; the settings button simply has no badge if this fails.
    getUpdate().then((status) => (update = status), () => undefined);
    let timer = setInterval(reload, POLL_MS);
    const onVisibility = () => {
      clearInterval(timer);
      if (document.visibilityState === "visible") {
        void reload();
        timer = setInterval(reload, POLL_MS);
      }
    };
    document.addEventListener("visibilitychange", onVisibility);
    return () => {
      clearInterval(timer);
      document.removeEventListener("visibilitychange", onVisibility);
    };
  });

  function onKeydown(event: KeyboardEvent) {
    const target = event.target as HTMLElement | null;
    const typing = target?.closest("input, textarea, [contenteditable]");
    if (!typing && (event.key === "/" || ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k"))) {
      event.preventDefault();
      search?.focus();
      search?.select();
    }
  }

  function onSearchKey(event: KeyboardEvent) {
    if (event.key === "Escape") {
      query = "";
      search?.blur();
    } else if (event.key === "Enter") {
      // Open the first visible result, services before bookmarks.
      const first = document.querySelector<HTMLAnchorElement>(".results a[href^='http']");
      if (first) window.open(first.href, "_blank", "noopener,noreferrer");
    }
  }

  const w = $derived(dash.widgets);
  const date = $derived(new Date(now.value));
  const dateLine = $derived(new Intl.DateTimeFormat("zh-CN", { month: "long", day: "numeric", weekday: "long" }).format(date));
  const time = $derived(new Intl.DateTimeFormat("zh-CN", { hour: "2-digit", minute: "2-digit", hour12: false }).format(date));

  const health = $derived.by(() => {
    const data = w.health?.data;
    if (!data) return null;
    const values = Object.values(data);
    return { up: values.filter((v) => v.up).length, total: values.length };
  });

  // Rows share their width between whatever is configured; a missing integration never leaves a hole.
  const row2 = $derived([w.vikunja && "todo", w.devices && "devices", w.homeassistant && "home"].filter(Boolean) as string[]);
  const row3 = $derived([w.forgejo && "commits", w.bookmarkd && "bookmarks", w.inbox && "inbox"].filter(Boolean) as string[]);
  const weights: Record<string, number> = { todo: 5, devices: 4, home: 3, commits: 5, bookmarks: 3, inbox: 4 };
  function span(row: string[], key: string): number {
    const total = row.reduce((sum, k) => sum + (weights[k] ?? 4), 0);
    return Math.round(((weights[key] ?? 4) / total) * 12);
  }

  // Section numbers follow document order, skipping what is not configured.
  const order = $derived(
    [dash.services.length > 0 && "services", ...row2, ...row3, w.github && "github"].filter(Boolean) as string[],
  );
  const index = (key: string) => order.indexOf(key) + 1;

  // The hero index: one line per headline figure, each jumping to its section.
  const headline = $derived.by(() => {
    const items: { key: string; label: string; value: number; of?: number; warn?: boolean }[] = [];
    if (health) items.push({ key: "services", label: "服务在线", value: health.up, of: health.total, warn: health.up < health.total });
    const devices = w.devices?.data;
    if (devices) items.push({ key: "devices", label: "IPv6 设备", value: devices.filter((d) => d.address).length, of: devices.length });
    const tasks = w.vikunja?.data?.tasks;
    if (tasks) {
      const today = new Date(now.value);
      const due = tasks.filter((t) => t.due && dayDiff(new Date(t.due), today) <= 0).length;
      items.push({ key: "todo", label: "今日待办", value: due, warn: due > 0 });
    }
    const recent = w.inbox?.data?.recent;
    if (recent) items.push({ key: "inbox", label: "收件箱", value: recent.length });
    return items;
  });
</script>

<svelte:window onkeydown={onKeydown} />

{#if settingsOpen}
  <SettingsDialog status={update} onstatus={(status) => (update = status)} onclose={() => (settingsOpen = false)} />
{/if}

<div class="page">
  <header class="mast">
    <a class="brand display" href="/">Home<span class="period">.</span></a>
    <p class="stamp label tnum"><span>{dateLine}</span><span class="time">{time}</span></p>
    <div class="controls">
      <label class="search">
        <Icon name="search" size={14} />
        <span class="visually-hidden">搜索服务和书签</span>
        <input
          bind:this={search}
          bind:value={query}
          onkeydown={onSearchKey}
          type="search"
          placeholder="搜索服务和书签"
          autocomplete="off"
          spellcheck="false"
        />
        {#if !query}<kbd>/</kbd>{/if}
      </label>
      <button
        class="tool label link"
        class:has-update={update?.available}
        onclick={() => (settingsOpen = true)}
        title={update?.available ? `有新版本 v${update.latest?.version}` : "设置"}
      >
        <Icon name="settings" size={14} /><span>{update?.available ? `v${update.latest?.version}` : "设置"}</span>
      </button>
      <button class="tool label link" onclick={signOut} title="退出登录">
        <Icon name="logout" size={14} /><span>退出</span>
      </button>
    </div>
  </header>

  <section class="hero">
    <div class="hello">
      <h1 class="display">{greeting(date)}{#if dash.profile.name}，<em>{dash.profile.name}</em>{/if}</h1>
      {#if dash.profile.tagline}<p class="tagline">{dash.profile.tagline}</p>{/if}
    </div>
    {#if headline.length > 0}
      <ul class="index">
        {#each headline as item (item.key)}
          <li>
            <a href="#section-{index(item.key)}">
              <span class="k"><i class="dot" class:down={item.warn}></i>{item.label}</span>
              <span class="n figure tnum">{item.value}{#if item.of !== undefined}<small>/{item.of}</small>{/if}</span>
            </a>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <main class="results">
    {#if dash.services.length > 0}
      <div class="row">
        <div class="cell" style:--span="12">
          <Card index={index("services")} title="服务" icon="box" slot={w.health} widget={w.health ? "health" : undefined} onrefreshed={reload}>
            {#snippet actions()}
              {#if health}
                <span class="pill" class:warn={health.up < health.total}><i class="dot" class:down={health.up < health.total}></i>{health.up}/{health.total} 在线</span>
              {/if}
            {/snippet}
            <Services services={dash.services} health={w.health} {query} />
          </Card>
        </div>
      </div>
    {/if}

    {#if row2.length > 0}
      <div class="row">
        {#each row2 as key (key)}
          <div class="cell" style:--span={span(row2, key)}>
            {#if key === "todo" && w.vikunja}
              <Card index={index(key)} title="今日待办" icon="todo" slot={w.vikunja} widget="vikunja" link={dash.links.vikunja} onrefreshed={reload}>
                {#if w.vikunja.data}<Todos data={w.vikunja.data} onchange={reload} />{/if}
              </Card>
            {:else if key === "devices" && w.devices}
              <Card index={index(key)} title="局域网 IPv6" icon="network" slot={w.devices} widget="devices" onrefreshed={reload}>
                {#if w.devices.data}<Devices devices={w.devices.data} />{/if}
              </Card>
            {:else if key === "home" && w.homeassistant}
              <Card index={index(key)} title="家" icon="house" slot={w.homeassistant} widget="homeassistant" link={dash.links.homeassistant} onrefreshed={reload}>
                {#if w.homeassistant.data}<Home entities={w.homeassistant.data.entities} />{/if}
              </Card>
            {/if}
          </div>
        {/each}
      </div>
    {/if}

    {#if row3.length > 0}
      <div class="row">
        {#each row3 as key (key)}
          <div class="cell" style:--span={span(row3, key)}>
            {#if key === "commits" && w.forgejo}
              <Card index={index(key)} title="最近提交" icon="branch" slot={w.forgejo} widget="forgejo" link={w.forgejo.data?.profileUrl ?? dash.links.forgejo} linkLabel="Forgejo" onrefreshed={reload}>
                {#if w.forgejo.data}<Commits data={w.forgejo.data} />{/if}
              </Card>
            {:else if key === "bookmarks" && w.bookmarkd}
              <Card index={index(key)} title="置顶书签" icon="bookmark" slot={w.bookmarkd} widget="bookmarkd" link={dash.links.bookmarkd} linkLabel="全部" onrefreshed={reload}>
                {#if w.bookmarkd.data}<Bookmarks items={w.bookmarkd.data.items} {query} />{/if}
              </Card>
            {:else if key === "inbox" && w.inbox}
              <!-- Sending works even while the status check is failing, so the form is always shown. -->
              <Card index={index(key)} title="收件箱" icon="inbox" widget="inbox" link={dash.links.inbox} onrefreshed={reload}>
                {#snippet actions()}
                  {#if w.inbox?.error}<span class="pill warn" title={w.inbox.error}><i class="dot down"></i>状态未知</span>{/if}
                {/snippet}
                <Inbox status={w.inbox.data} link={dash.links.inbox} onsent={reload} />
              </Card>
            {/if}
          </div>
        {/each}
      </div>
    {/if}

    {#if w.github}
      <div class="row">
        <div class="cell" style:--span="12">
          <Card index={index("github")} title="GitHub" icon="github" slot={w.github} widget="github" link={dash.links.github} linkLabel={w.github.data ? `@${w.github.data.user.login}` : "打开"} onrefreshed={reload}>
            {#if w.github.data}<Github data={w.github.data} />{/if}
          </Card>
        </div>
      </div>
    {/if}
  </main>

  <footer class="foot label">
    <span>Homepage{#if update?.current}&nbsp;v{update.current}{/if}</span>
    <span class="tnum">{dateLine} · {time}</span>
  </footer>
</div>

<style>
  .page {
    position: relative;
    z-index: 1;
    width: min(100%, 1320px);
    margin: 0 auto;
    padding: clamp(18px, 2.5vw, 28px) var(--gutter) calc(40px + env(safe-area-inset-bottom));
  }

  /* ------------------------------------------------------------ masthead */
  .mast {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 24px;
    padding-bottom: 12px;
    border-bottom: 1px solid var(--section-rule);
  }
  :global(:root[data-theme="panel"]) .mast {
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 6px 6px 6px 16px;
    box-shadow: var(--card-shadow);
    min-height: 46px;
  }
  :global(:root[data-theme="swiss"]) .mast {
    border-top: 2px solid var(--line);
    border-bottom: 1px solid var(--line);
    padding: 10px 0;
  }

  .brand {
    font-size: 20px;
    font-style: italic;
    line-height: 1;
  }
  .brand .period {
    color: var(--accent);
  }
  :global(:root[data-theme="panel"]) .brand,
  :global(:root[data-theme="swiss"]) .brand {
    font-style: normal;
    font-family: var(--font-mono);
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.14em;
    text-transform: uppercase;
  }

  .stamp {
    display: flex;
    gap: 14px;
    margin: 0;
    color: var(--muted);
  }
  :global(:root[data-theme="panel"]) .stamp .time {
    color: var(--accent);
    font-size: 13px;
    letter-spacing: 0.04em;
  }

  .controls {
    display: flex;
    align-items: center;
    gap: 22px;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 240px;
    height: 32px;
    padding: 0 2px;
    color: var(--muted);
    border-bottom: 1px solid var(--hair);
    transition: border-color 160ms ease;
  }
  .search:focus-within {
    border-color: var(--ink);
    color: var(--ink);
  }
  :global(:root[data-theme="panel"]) .search,
  :global(:root[data-theme="swiss"]) .search {
    width: 280px;
    height: 34px;
    padding: 0 10px;
    border: var(--card-border-w) solid var(--input-border);
    border-radius: var(--input-radius);
    background: var(--input-bg);
  }
  :global(:root[data-theme="panel"]) .search:focus-within,
  :global(:root[data-theme="swiss"]) .search:focus-within {
    border-color: var(--accent);
  }
  .search input {
    flex: 1;
    min-width: 0;
    height: 100%;
    border: 0;
    outline: none;
    background: transparent;
    font-size: 16px; /* prevents iOS zoom on focus */
    color: var(--ink);
  }
  @media (hover: hover) and (pointer: fine) {
    .search input {
      font-size: 13px;
    }
  }
  .search input::placeholder {
    color: var(--muted);
  }
  .search input::-webkit-search-cancel-button {
    -webkit-appearance: none;
    appearance: none;
  }
  kbd {
    display: grid;
    place-items: center;
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    border: 1px solid var(--hair);
    border-radius: var(--chip-radius);
    font-family: var(--font-mono);
    font-size: 10.5px;
    color: var(--muted);
  }
  @media (hover: none) {
    kbd {
      display: none;
    }
  }
  .tool {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
    transition: color 160ms ease;
  }
  @media (hover: hover) and (pointer: fine) {
    .tool:hover {
      color: var(--ink);
    }
  }
  .tool.has-update {
    color: var(--accent);
  }

  /* ---------------------------------------------------------------- hero */
  .hero {
    display: grid;
    grid-template-columns: minmax(0, 1.4fr) minmax(300px, 1fr);
    gap: 48px;
    align-items: end;
    padding: clamp(36px, 5vw, 60px) 0 clamp(28px, 3vw, 40px);
    border-bottom: 1px solid var(--section-rule);
  }
  :global(:root[data-theme="panel"]) .hero {
    padding: clamp(28px, 4vw, 44px) 8px 28px;
    border-bottom: 0;
  }
  :global(:root[data-theme="swiss"]) .hero {
    border-bottom: 0;
    padding-bottom: 28px;
  }

  h1 {
    margin: 0;
    font-size: var(--size-display);
    line-height: var(--lh-display);
    text-wrap: balance;
  }
  h1 em {
    font-style: italic;
    color: var(--accent);
  }
  :global(:root[data-theme="panel"]) h1 em {
    font-style: normal;
    color: inherit;
  }
  :global(:root[data-theme="swiss"]) h1 em {
    font-style: normal;
    color: var(--accent);
  }
  .tagline {
    margin: 20px 0 0;
    font-size: 16px;
    color: var(--ink-2);
    max-width: 42ch;
  }
  :global(:root[data-theme="panel"]) .tagline {
    font-size: 13px;
    font-family: var(--font-mono);
    color: var(--muted);
    margin-top: 14px;
  }

  /* The index: paper rules it, panel segments it, swiss boxes it. */
  .index {
    list-style: none;
    margin: 0;
    padding: 0;
    border-top: 1px solid var(--hair);
  }
  .index li {
    border-bottom: 1px solid var(--hair);
  }
  .index a {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    padding: 11px 0;
    gap: 16px;
    transition: padding-left 220ms var(--ease-out), color 160ms ease;
  }
  @media (hover: hover) and (pointer: fine) {
    .index a:hover {
      padding-left: 6px;
      color: var(--accent-ink);
    }
  }
  .index .k {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13.5px;
  }
  .index .n {
    font-size: 30px;
  }
  .index .n small {
    font-size: 0.55em;
    color: var(--muted);
    margin-left: 1px;
    font-weight: inherit;
  }
  :global(:root[data-theme="panel"]) .index,
  :global(:root[data-theme="swiss"]) .index {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    border: var(--card-border-w) solid var(--card-border);
    border-radius: var(--card-radius);
    background: var(--card-bg);
    box-shadow: var(--card-shadow);
    overflow: hidden;
  }
  :global(:root[data-theme="panel"]) .index li,
  :global(:root[data-theme="swiss"]) .index li {
    border-bottom: 0;
    border-right: var(--card-border-w) solid var(--card-border);
  }
  :global(:root[data-theme="panel"]) .index li:last-child,
  :global(:root[data-theme="swiss"]) .index li:last-child {
    border-right: 0;
  }
  :global(:root[data-theme="panel"]) .index a,
  :global(:root[data-theme="swiss"]) .index a {
    flex-direction: column-reverse;
    align-items: flex-start;
    gap: 6px;
    padding: 14px 14px 12px;
  }
  :global(:root[data-theme="panel"]) .index a:hover,
  :global(:root[data-theme="swiss"]) .index a:hover {
    padding-left: 14px;
    background: var(--surface-hover);
  }
  :global(:root[data-theme="panel"]) .index .k,
  :global(:root[data-theme="swiss"]) .index .k {
    font-family: var(--font-mono);
    font-size: 10.5px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--muted);
  }
  :global(:root[data-theme="panel"]) .index .n {
    font-size: 34px;
    color: var(--accent);
  }
  :global(:root[data-theme="swiss"]) .index .n {
    font-size: 40px;
  }
  :global(:root[data-theme="swiss"]) .index li a .dot {
    display: none;
  }

  /* ---------------------------------------------------------------- rows */
  .results {
    display: grid;
    gap: 0;
  }
  :global(:root[data-theme="panel"]) .results,
  :global(:root[data-theme="swiss"]) .results {
    gap: 14px;
  }
  .row {
    display: grid;
    grid-template-columns: repeat(12, minmax(0, 1fr));
    padding: 28px 0 32px;
    border-top: 1px solid var(--section-rule);
  }
  .row + .row {
    border-top-color: var(--section-rule);
  }
  :global(:root[data-theme="panel"]) .row,
  :global(:root[data-theme="swiss"]) .row {
    padding: 0;
    border-top: 0;
    gap: 14px;
  }
  .cell {
    grid-column: span var(--span);
    min-width: 0;
    padding: 0 36px;
    animation: enter 460ms var(--ease-out) both;
  }
  .cell:first-child {
    padding-left: 0;
  }
  .cell:last-child {
    padding-right: 0;
  }
  .cell + .cell {
    border-left: 1px solid var(--col-rule);
  }
  :global(:root[data-theme="panel"]) .cell,
  :global(:root[data-theme="swiss"]) .cell {
    padding: 0;
    border-left: 0;
  }
  .row:nth-child(2) .cell {
    animation-delay: 60ms;
  }
  .row:nth-child(3) .cell {
    animation-delay: 120ms;
  }
  .row:nth-child(n + 4) .cell {
    animation-delay: 180ms;
  }
  @keyframes enter {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }

  .pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--ok);
    white-space: nowrap;
  }
  .pill.warn {
    color: var(--warn);
  }

  .foot {
    display: flex;
    justify-content: space-between;
    margin-top: 40px;
    padding-top: 12px;
    border-top: 1px solid var(--section-rule);
  }
  :global(:root[data-theme="panel"]) .foot {
    border-top-color: var(--hair);
  }
  :global(:root[data-theme="swiss"]) .foot {
    border-top-width: 2px;
  }

  /* ------------------------------------------------------------ narrow */
  @media (max-width: 1080px) {
    .cell {
      grid-column: span max(6, var(--span));
    }
    .cell:nth-child(3) {
      grid-column: span 12;
      border-left: 0;
      border-top: 1px solid var(--col-rule);
      padding: 28px 0 0;
      margin-top: 28px;
    }
    :global(:root[data-theme="panel"]) .cell:nth-child(3),
    :global(:root[data-theme="swiss"]) .cell:nth-child(3) {
      border-top: 0;
      padding: 0;
      margin-top: 0;
    }
    .cell:nth-child(2) {
      padding-right: 0;
    }
    .cell:nth-child(1):not(:only-child) {
      padding-right: 28px;
    }
    .cell:nth-child(2) {
      padding-left: 28px;
    }
  }
  @media (max-width: 760px) {
    .mast {
      grid-template-columns: 1fr auto;
      row-gap: 12px;
    }
    .stamp {
      display: none;
    }
    .controls {
      gap: 16px;
    }
    .search {
      grid-column: 1 / -1;
      width: 100%;
      order: 3;
    }
    :global(:root[data-theme="panel"]) .mast {
      padding: 10px 12px;
    }
    .tool span {
      display: none;
    }
    .hero {
      grid-template-columns: 1fr;
      gap: 24px;
      padding-top: 28px;
    }
    :global(:root[data-theme="panel"]) .index,
    :global(:root[data-theme="swiss"]) .index {
      grid-template-columns: 1fr 1fr;
    }
    :global(:root[data-theme="panel"]) .index li:nth-child(2),
    :global(:root[data-theme="swiss"]) .index li:nth-child(2) {
      border-right: 0;
    }
    :global(:root[data-theme="panel"]) .index li:nth-child(-n + 2),
    :global(:root[data-theme="swiss"]) .index li:nth-child(-n + 2) {
      border-bottom: var(--card-border-w) solid var(--card-border);
    }
    .row {
      padding: 22px 0 24px;
    }
    .cell,
    .cell:nth-child(n) {
      grid-column: span 12;
      padding: 0;
      border-left: 0;
      margin-top: 0;
    }
    .cell + .cell {
      border-top: 1px solid var(--col-rule);
      padding-top: 22px;
      margin-top: 22px;
    }
    :global(:root[data-theme="panel"]) .cell + .cell,
    :global(:root[data-theme="swiss"]) .cell + .cell {
      border-top: 0;
      padding-top: 0;
      margin-top: 0;
    }
    .foot {
      flex-direction: column;
      gap: 6px;
    }
  }
</style>

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
  import UpdateDialog from "./UpdateDialog.svelte";
  import { ApiError, getDashboard, getUpdate, logout, type Dashboard, type UpdateStatus } from "./lib/api";
  import { greeting } from "./lib/time";
  import { now, startClock } from "./lib/clock.svelte";

  let { initial, onsignedout }: { initial: Dashboard; onsignedout: () => void } = $props();

  // svelte-ignore state_referenced_locally
  let dash = $state<Dashboard>(initial);
  let query = $state("");
  let search: HTMLInputElement | undefined = $state();
  let update = $state<UpdateStatus | null>(null);
  let updatesOpen = $state(false);

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
    // Updates are optional; the button simply has no badge if this fails.
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
      const first = document.querySelector<HTMLAnchorElement>(".results a[href]");
      if (first) window.open(first.href, "_blank", "noopener,noreferrer");
    }
  }

  const w = $derived(dash.widgets);
  const date = $derived(new Date(now.value));
  const dateLine = $derived(
    new Intl.DateTimeFormat("zh-CN", { month: "long", day: "numeric", weekday: "long" }).format(date),
  );
  const time = $derived(new Intl.DateTimeFormat("zh-CN", { hour: "2-digit", minute: "2-digit", hour12: false }).format(date));

  const health = $derived.by(() => {
    const data = w.health?.data;
    if (!data) return null;
    const values = Object.values(data);
    return { up: values.filter((v) => v.up).length, total: values.length };
  });

  // Cards in a row share its width, so a missing integration never leaves a hole.
  const row2 = $derived([w.vikunja && "todo", w.devices && "devices", w.homeassistant && "home"].filter(Boolean) as string[]);
  const row3 = $derived([w.forgejo && "commits", w.bookmarkd && "bookmarks", w.inbox && "inbox"].filter(Boolean) as string[]);
  const span = (row: string[]) => 12 / Math.max(1, row.length);
</script>

<svelte:window onkeydown={onKeydown} />

{#if updatesOpen && update}
  <UpdateDialog status={update} onstatus={(status) => (update = status)} onclose={() => (updatesOpen = false)} />
{/if}

<div class="page">
  <header class="top">
    <div class="hello">
      <p class="date">
        <span>{dateLine}</span>
        <span class="sep" aria-hidden="true"></span>
        <span class="tnum">{time}</span>
      </p>
      <h1>{greeting(date)}{dash.profile.name ? `，${dash.profile.name}` : ""}</h1>
      {#if dash.profile.tagline}<p class="tagline">{dash.profile.tagline}</p>{/if}
    </div>
    <div class="controls">
      <label class="search">
        <Icon name="search" size={15} />
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
      {#if update}
        <button
          class="icon-button pressable"
          class:has-update={update.available}
          onclick={() => (updatesOpen = true)}
          title={update.available ? `有新版本 v${update.latest?.version}` : `版本 v${update.current}`}
          aria-label={update.available ? `设置，有新版本 v${update.latest?.version}` : "设置"}
        >
          <Icon name="settings" size={16} />
          {#if update.available}<span class="update-label">v{update.latest?.version}</span>{/if}
        </button>
      {/if}
      <button class="icon-button pressable" onclick={signOut} title="退出登录" aria-label="退出登录">
        <Icon name="logout" size={16} />
      </button>
    </div>
  </header>

  <main class="grid results">
    {#if dash.services.length > 0}
      <div class="cell" style:--span="12">
        <Card title="服务" icon="box" slot={w.health} widget={w.health ? "health" : undefined} onrefreshed={reload}>
          {#snippet actions()}
            {#if health}
              <span class="pill" class:warn={health.up < health.total}>
                <i></i>{health.up}/{health.total} 在线
              </span>
            {/if}
          {/snippet}
          <Services services={dash.services} health={w.health} {query} />
        </Card>
      </div>
    {/if}

    {#each row2 as key (key)}
      <div class="cell" style:--span={span(row2)}>
        {#if key === "todo" && w.vikunja}
          <Card title="今日待办" icon="todo" slot={w.vikunja} widget="vikunja" link={dash.links.vikunja} onrefreshed={reload}>
            {#if w.vikunja.data}<Todos data={w.vikunja.data} onchange={reload} />{/if}
          </Card>
        {:else if key === "devices" && w.devices}
          <Card title="局域网 IPv6" icon="network" slot={w.devices} widget="devices" onrefreshed={reload}>
            {#if w.devices.data}<Devices devices={w.devices.data} />{/if}
          </Card>
        {:else if key === "home" && w.homeassistant}
          <Card title="家" icon="house" slot={w.homeassistant} widget="homeassistant" link={dash.links.homeassistant} onrefreshed={reload}>
            {#if w.homeassistant.data}<Home entities={w.homeassistant.data.entities} />{/if}
          </Card>
        {/if}
      </div>
    {/each}

    {#each row3 as key (key)}
      <div class="cell" style:--span={span(row3)}>
        {#if key === "commits" && w.forgejo}
          <Card title="最近提交" icon="branch" slot={w.forgejo} widget="forgejo" link={w.forgejo.data?.profileUrl ?? dash.links.forgejo} onrefreshed={reload}>
            {#if w.forgejo.data}<Commits data={w.forgejo.data} />{/if}
          </Card>
        {:else if key === "bookmarks" && w.bookmarkd}
          <Card title="置顶书签" icon="bookmark" slot={w.bookmarkd} widget="bookmarkd" link={dash.links.bookmarkd} onrefreshed={reload}>
            {#if w.bookmarkd.data}<Bookmarks items={w.bookmarkd.data.items} {query} />{/if}
          </Card>
        {:else if key === "inbox" && w.inbox}
          <!-- Sending works even while the status check is failing, so the form is always shown. -->
          <Card title="收件箱" icon="inbox" widget="inbox" link={dash.links.inbox} onrefreshed={reload}>
            {#snippet actions()}
              {#if w.inbox?.error}
                <span class="pill warn" title={w.inbox.error}><i></i>状态未知</span>
              {/if}
            {/snippet}
            <Inbox status={w.inbox.data} link={dash.links.inbox} onsent={reload} />
          </Card>
        {/if}
      </div>
    {/each}

    {#if w.github}
      <div class="cell" style:--span="12">
        <Card title="GitHub" icon="github" slot={w.github} widget="github" link={dash.links.github} onrefreshed={reload}>
          {#if w.github.data}<Github data={w.github.data} />{/if}
        </Card>
      </div>
    {/if}
  </main>
</div>

<style>
  .page {
    width: min(100%, 1240px);
    margin: 0 auto;
    padding: clamp(28px, 6vw, 64px) clamp(16px, 4vw, 40px) calc(48px + env(safe-area-inset-bottom));
  }

  .top {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 24px;
    margin-bottom: 28px;
  }
  @media (max-width: 760px) {
    .top {
      flex-direction: column;
      align-items: stretch;
      gap: 20px;
      margin-bottom: 20px;
    }
  }

  .date {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0 0 8px;
    font-size: 13px;
    color: var(--muted);
  }
  .sep {
    width: 3px;
    height: 3px;
    border-radius: 50%;
    background: var(--faint);
  }
  h1 {
    margin: 0;
    font-size: clamp(28px, 4.2vw, 40px);
    font-weight: 620;
    letter-spacing: -0.035em;
    line-height: 1.08;
    text-wrap: balance;
  }
  .tagline {
    margin: 10px 0 0;
    color: var(--ink-2);
    font-size: 14.5px;
    max-width: 60ch;
  }

  .controls {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    width: min(100%, 300px);
    height: 40px;
    padding: 0 10px 0 13px;
    border-radius: 12px;
    border: 1px solid var(--line-strong);
    background: var(--surface);
    color: var(--muted);
    box-shadow: var(--shadow-card);
    transition:
      border-color 160ms ease,
      box-shadow 160ms ease;
  }
  @media (max-width: 760px) {
    .search {
      flex: 1;
      width: auto;
    }
  }
  .search:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
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
      font-size: 14px;
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
    min-width: 20px;
    height: 20px;
    padding: 0 5px;
    border-radius: 6px;
    border: 1px solid var(--line-strong);
    background: var(--surface-2);
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--muted);
  }
  @media (hover: none) {
    kbd {
      display: none;
    }
  }

  .icon-button {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border-radius: 12px;
    border: 1px solid var(--line-strong);
    background: var(--surface);
    color: var(--muted);
    box-shadow: var(--shadow-card);
  }
  @media (hover: hover) and (pointer: fine) {
    .icon-button:hover {
      color: var(--ink);
      background: var(--surface-hover);
    }
  }

  .icon-button.has-update {
    position: relative;
    display: inline-flex;
    gap: 7px;
    width: auto;
    padding: 0 12px;
    color: var(--accent-ink);
    border-color: color-mix(in srgb, var(--accent) 35%, transparent);
    background: var(--accent-soft);
  }
  .icon-button.has-update::after {
    content: "";
    position: absolute;
    top: -3px;
    right: -3px;
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--accent);
    border: 2px solid var(--bg);
  }
  .update-label {
    font-family: var(--font-mono);
    font-size: 12.5px;
    font-weight: 550;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(12, minmax(0, 1fr));
    gap: 14px;
  }
  .cell {
    grid-column: span var(--span);
    display: grid;
    min-width: 0;
    animation: enter 420ms var(--ease-out) both;
  }
  .cell:nth-child(2) {
    animation-delay: 40ms;
  }
  .cell:nth-child(3) {
    animation-delay: 80ms;
  }
  .cell:nth-child(4) {
    animation-delay: 120ms;
  }
  .cell:nth-child(n + 5) {
    animation-delay: 160ms;
  }
  @keyframes enter {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
  }
  /* Three across gets cramped below desktop widths: stack them in pairs, then singly. */
  @media (max-width: 1080px) {
    .cell {
      grid-column: span max(6, var(--span));
    }
    .cell:last-child:nth-child(odd) {
      grid-column: span 12;
    }
  }
  @media (max-width: 760px) {
    .grid {
      gap: 12px;
    }
    .cell,
    .cell:last-child:nth-child(odd) {
      grid-column: span 12;
    }
  }

  .pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-right: 8px;
    padding: 2px 9px 2px 8px;
    border-radius: 999px;
    font-size: 12px;
    font-weight: 500;
    color: var(--accent-ink);
    background: var(--accent-soft);
    white-space: nowrap;
  }
  .pill i {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
  }
  .pill.warn {
    color: var(--warn);
    background: var(--warn-soft);
  }
</style>

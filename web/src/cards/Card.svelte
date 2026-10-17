<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "../lib/Icon.svelte";
  import { ago } from "../lib/time";
  import type { Slot } from "../lib/api";
  import { refreshWidget } from "../lib/api";
  import { now } from "../lib/clock.svelte";

  /** A numbered section. Paper rules it off, panel frames it as a module,
   *  swiss boxes it on a tinted field; the markup is the same. */
  let {
    index,
    title,
    icon,
    slot,
    widget,
    link,
    linkLabel = "打开",
    children,
    actions,
    onrefreshed,
  }: {
    index: number;
    title: string;
    icon: string;
    slot?: Slot<unknown>;
    widget?: string;
    link?: string;
    linkLabel?: string;
    children: Snippet;
    actions?: Snippet;
    onrefreshed?: () => void;
  } = $props();

  let spinning = $state(false);

  async function refresh() {
    if (!widget || spinning) return;
    spinning = true;
    try {
      await refreshWidget(widget);
      // Give the background fetcher a moment, then reload the dashboard.
      await new Promise((resolve) => setTimeout(resolve, 900));
      onrefreshed?.();
    } finally {
      spinning = false;
    }
  }

  const stale = $derived(!!slot?.error && !!slot?.data);
  const num = $derived(String(index).padStart(2, "0"));
</script>

<section class="card" style:--field="var(--field-{index})" aria-labelledby="section-{index}">
  <header>
    <div class="title">
      <span class="label num">{num}</span>
      <h2 id="section-{index}">{title}</h2>
      <span class="glyph" aria-hidden="true"><Icon name={icon} size={14} /></span>
      {#if slot?.error}
        <span class="badge" class:stale title={slot.error}>
          <Icon name="alert" size={11} stroke={2} />
          {stale ? "可能过期" : "无法连接"}
        </span>
      {/if}
    </div>
    <div class="tools label">
      {@render actions?.()}
      {#if slot?.updatedAt && !slot.error}
        <span class="updated tnum" title="上次更新">{ago(slot.updatedAt, now.value)}</span>
      {/if}
      {#if widget}
        <button class="tool link" class:spinning onclick={refresh} aria-label="刷新{title}">
          <Icon name="refresh" size={12} /><span>刷新</span>
        </button>
      {/if}
      {#if link}
        <a class="tool link" href={link} target="_blank" rel="noopener noreferrer" aria-label="{linkLabel}{title}">
          <span>{linkLabel}</span><Icon name="arrow-up-right" size={12} />
        </a>
      {/if}
    </div>
  </header>
  <div class="body">
    {#if slot && !slot.data && !slot.error}
      <div class="skeleton" aria-hidden="true">
        <span style="width: 72%"></span><span style="width: 54%"></span><span style="width: 63%"></span>
      </div>
    {:else if slot && !slot.data && slot.error}
      <div class="failure">
        <Icon name="alert" size={18} />
        <p>{slot.error}</p>
      </div>
    {:else}
      {@render children()}
    {/if}
  </div>
</section>

<style>
  .card {
    position: relative;
    display: flex;
    flex-direction: column;
    min-width: 0;
    height: 100%;
    container-type: inline-size;
    scroll-margin-top: 24px;
    background: var(--card-bg);
    border: var(--card-border-w) solid var(--card-border);
    border-radius: var(--card-radius);
    box-shadow: var(--card-shadow);
  }
  /* Swiss paints each section on its own tinted field. */
  :global(:root[data-theme="swiss"]) .card {
    background: var(--field, var(--surface));
  }

  header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 16px;
    padding: var(--card-pad-y) var(--card-pad-x) 0;
    margin-bottom: 20px;
  }
  :global(:root[data-family="panel"]) header,
  :global(:root[data-theme="swiss"]) header {
    align-items: center;
    padding-bottom: var(--card-pad-y);
    border-bottom: var(--card-border-w) solid var(--head-rule);
    margin-bottom: 16px;
  }
  :global(:root[data-theme="swiss"]) header {
    border-bottom-color: var(--line);
  }

  .title {
    display: flex;
    align-items: baseline;
    gap: 14px;
    min-width: 0;
  }
  :global(:root[data-family="panel"]) .title,
  :global(:root[data-theme="swiss"]) .title {
    align-items: center;
    gap: 10px;
  }
  .num {
    color: var(--muted);
  }
  :global(:root[data-family="panel"]) .num::before {
    content: "";
    display: inline-block;
    width: 6px;
    height: 6px;
    background: var(--accent);
    margin-right: 10px;
    vertical-align: 1px;
  }

  h2 {
    margin: 0;
    font-family: var(--font-heading);
    font-weight: var(--w-heading);
    font-size: var(--size-heading);
    letter-spacing: var(--ls-heading);
    line-height: 1;
    white-space: nowrap;
  }
  :global(:root[data-family="panel"]) h2 {
    text-transform: uppercase;
    color: var(--ink-2);
  }

  /* The icon is a quiet secondary mark; paper hides it (the serif title is the mark). */
  .glyph {
    display: none;
    color: var(--muted);
  }
  :global(:root[data-family="panel"]) .glyph,
  :global(:root[data-theme="swiss"]) .glyph {
    display: inline-flex;
  }

  .badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 7px 2px 6px;
    border-radius: var(--chip-radius);
    font-size: 11px;
    font-weight: 500;
    color: var(--danger);
    background: var(--danger-soft);
    white-space: nowrap;
    cursor: help;
  }
  .badge.stale {
    color: var(--warn);
    background: var(--warn-soft);
  }

  .tools {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    flex-wrap: wrap;
    gap: 6px 18px;
    white-space: nowrap;
  }
  /* Narrow columns: the title shrinks a step and the tools drop below it. */
  @container (max-width: 380px) {
    header {
      flex-wrap: wrap;
      row-gap: 10px;
    }
    h2 {
      font-size: calc(var(--size-heading) * 0.8);
    }
    .tools {
      flex-basis: 100%;
      justify-content: flex-start;
    }
  }
  .updated {
    color: var(--muted);
  }
  .tool {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: var(--muted);
    font: inherit;
    letter-spacing: inherit;
    text-transform: inherit;
    transition: color 160ms ease;
  }
  @media (hover: hover) and (pointer: fine) {
    .tool:hover {
      color: var(--ink);
    }
  }
  .tool.spinning :global(svg) {
    animation: spin 700ms linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  /* On phones the labels crowd the header: keep the icons only. */
  @media (max-width: 480px) {
    .tool span {
      display: none;
    }
    .tools {
      gap: 14px;
    }
  }

  .body {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 0 var(--card-pad-x) var(--card-pad-x);
  }
  :global(:root[data-theme="paper"]) .body,
  :global(:root:not([data-theme])) .body {
    padding-bottom: 8px;
  }

  .skeleton {
    display: grid;
    gap: 12px;
    padding: 6px 0;
  }
  .skeleton span {
    height: 11px;
    background: linear-gradient(90deg, var(--heat-0), var(--hair), var(--heat-0));
    background-size: 200% 100%;
    animation: shimmer 1.4s ease-in-out infinite;
  }
  @keyframes shimmer {
    from {
      background-position: 100% 0;
    }
    to {
      background-position: -100% 0;
    }
  }

  .failure {
    flex: 1;
    display: grid;
    place-items: center;
    align-content: center;
    gap: 8px;
    min-height: 120px;
    color: var(--muted);
    text-align: center;
  }
  .failure p {
    margin: 0;
    font-size: 13px;
    max-width: 32ch;
  }
</style>

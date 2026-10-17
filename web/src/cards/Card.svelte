<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "../lib/Icon.svelte";
  import { ago } from "../lib/time";
  import type { Slot } from "../lib/api";
  import { refreshWidget } from "../lib/api";
  import { now } from "../lib/clock.svelte";

  let {
    title,
    icon,
    slot,
    widget,
    link,
    linkLabel = "打开",
    area,
    children,
    actions,
    onrefreshed,
  }: {
    title: string;
    icon: string;
    slot?: Slot<unknown>;
    widget?: string;
    link?: string;
    linkLabel?: string;
    area?: string;
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
</script>

<section class="card" style:grid-area={area} aria-label={title}>
  <header>
    <div class="title">
      <span class="glyph"><Icon name={icon} size={15} /></span>
      <h2>{title}</h2>
      {#if slot?.error}
        <span class="badge" class:stale title={slot.error}>
          <Icon name="alert" size={12} stroke={2} />
          {stale ? "数据可能过期" : "无法连接"}
        </span>
      {/if}
    </div>
    <div class="tools">
      {@render actions?.()}
      {#if slot?.updatedAt && !slot.error}
        <span class="updated tnum" title="上次更新">{ago(slot.updatedAt, now.value)}</span>
      {/if}
      {#if widget}
        <button class="tool pressable" class:spinning onclick={refresh} aria-label="刷新{title}" title="刷新">
          <Icon name="refresh" size={14} />
        </button>
      {/if}
      {#if link}
        <a class="tool pressable" href={link} target="_blank" rel="noopener noreferrer" aria-label="{linkLabel}{title}" title={linkLabel}>
          <Icon name="arrow-up-right" size={15} />
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
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: var(--radius-card);
    box-shadow: var(--shadow-card);
    overflow: hidden;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 14px 14px 0 18px;
    min-height: 46px;
  }

  .title {
    display: flex;
    align-items: center;
    gap: 9px;
    min-width: 0;
  }

  .glyph {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: 8px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    color: var(--ink-2);
  }

  h2 {
    margin: 0;
    font-size: 13.5px;
    font-weight: 600;
    letter-spacing: -0.005em;
    white-space: nowrap;
  }

  .badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 7px 2px 6px;
    border-radius: 999px;
    font-size: 11.5px;
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
    gap: 2px;
    color: var(--muted);
  }

  .updated {
    font-size: 12px;
    margin-right: 6px;
    white-space: nowrap;
  }

  .tool {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: 9px;
    color: var(--muted);
  }
  @media (hover: hover) and (pointer: fine) {
    .tool:hover {
      background: var(--surface-hover);
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

  .body {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 12px 18px 18px;
  }

  .skeleton {
    display: grid;
    gap: 12px;
    padding: 6px 0;
  }
  .skeleton span {
    height: 12px;
    border-radius: 6px;
    background: linear-gradient(90deg, var(--heat-0), var(--line-strong), var(--heat-0));
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

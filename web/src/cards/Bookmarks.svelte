<script lang="ts">
  import type { Bookmark } from "../lib/api";

  let { items, query = "" }: { items: Bookmark[]; query?: string } = $props();

  const visible = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return items;
    return items.filter((item) => [item.title, item.host, ...item.tags].some((field) => field.toLowerCase().includes(q)));
  });

  /** A stable, gentle hue per host for monogram tiles. */
  function hue(text: string) {
    let hash = 0;
    for (const char of text) hash = (hash * 31 + char.charCodeAt(0)) | 0;
    return Math.abs(hash) % 360;
  }
</script>

{#if items.length === 0}
  <p class="empty">在 bookmarkd 中置顶书签后会显示在这里</p>
{:else if visible.length === 0}
  <p class="empty">没有匹配的书签</p>
{:else}
  <ul class="grid">
    {#each visible as item (item.id)}
      <li>
        <a class="item pressable" href={item.url} target="_blank" rel="noopener noreferrer" title={item.url}>
          {#if item.icon}
            <img class="favicon" src={item.icon} alt="" width="20" height="20" loading="lazy" />
          {:else}
            <span class="favicon monogram" style:--h={hue(item.host)}>{Array.from(item.title.trim())[0]?.toUpperCase() ?? "#"}</span>
          {/if}
          <span class="text">
            <span class="title">{item.title}</span>
            <span class="host">{item.host}</span>
          </span>
        </a>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .grid {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(188px, 1fr));
    gap: 4px 8px;
  }

  .item {
    display: flex;
    align-items: center;
    gap: 11px;
    padding: 8px 10px;
    margin: 0 -2px;
    border-radius: 11px;
  }
  @media (hover: hover) and (pointer: fine) {
    .item:hover {
      background: var(--surface-hover);
    }
  }

  .favicon {
    flex: none;
    width: 28px;
    height: 28px;
    padding: 4px;
    border-radius: 8px;
    background: var(--surface);
    border: 1px solid var(--line);
    object-fit: contain;
  }
  .monogram {
    display: grid;
    place-items: center;
    padding: 0;
    font-size: 12.5px;
    font-weight: 650;
    color: oklch(0.45 0.09 var(--h));
    background: oklch(0.95 0.03 var(--h));
    border-color: transparent;
  }
  @media (prefers-color-scheme: dark) {
    .monogram {
      color: oklch(0.82 0.08 var(--h));
      background: oklch(0.3 0.04 var(--h));
    }
  }

  .text {
    display: grid;
    min-width: 0;
  }
  .title {
    font-size: 13.5px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .host {
    font-size: 12px;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .empty {
    margin: 4px 0;
    color: var(--muted);
    font-size: 13px;
  }
</style>

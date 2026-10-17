<script lang="ts">
  import type { Bookmark } from "../lib/api";

  let { items, query = "" }: { items: Bookmark[]; query?: string } = $props();

  const visible = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return items;
    return items.filter((item) => [item.title, item.host, ...item.tags].some((field) => field.toLowerCase().includes(q)));
  });
</script>

{#if items.length === 0}
  <p class="empty">在 bookmarkd 中置顶书签后会显示在这里</p>
{:else if visible.length === 0}
  <p class="empty">没有匹配的书签</p>
{:else}
  <ul>
    {#each visible as item (item.id)}
      <li>
        <a class="item" href={item.url} target="_blank" rel="noopener noreferrer" title={item.url}>
          {#if item.icon}
            <img class="mark" src={item.icon} alt="" width="18" height="18" loading="lazy" />
          {:else}
            <span class="mark monogram figure">{Array.from(item.title.trim())[0]?.toUpperCase() ?? "#"}</span>
          {/if}
          <span class="text">
            <span class="title">{item.title}</span>
            <span class="host mono">{item.host}</span>
          </span>
        </a>
      </li>
    {/each}
  </ul>
{/if}

<style>
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li {
    border-top: 1px solid var(--hair-soft);
  }
  li:first-child {
    border-top: 0;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 0;
    transition: padding-left 220ms var(--ease-out), color 160ms ease;
  }
  @media (hover: hover) and (pointer: fine) {
    .item:hover {
      padding-left: 4px;
      color: var(--accent-ink);
    }
  }

  /* Paper: an italic initial in a ring. Panel: a keycap. Swiss: a black square. */
  .mark {
    flex: none;
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border: 1px solid var(--hair);
    border-radius: 50%;
    object-fit: contain;
    padding: 0;
  }
  img.mark {
    padding: 4px;
    background: var(--surface);
  }
  .monogram {
    font-size: 15px;
    font-style: italic;
    color: var(--ink);
  }
  :global(:root[data-theme="panel"]) .mark {
    border-radius: 5px;
    background: var(--bg);
    border-color: var(--line);
  }
  :global(:root[data-theme="panel"]) .monogram {
    font-style: normal;
    font-family: var(--font-mono);
    font-weight: 600;
    font-size: 12px;
    color: var(--ink-2);
  }
  :global(:root[data-theme="swiss"]) .mark {
    border-radius: 0;
    border: 0;
    background: var(--ink);
  }
  :global(:root[data-theme="swiss"]) .monogram {
    font-style: normal;
    font-size: 12px;
    color: var(--bg);
  }
  :global(:root[data-theme="swiss"]) img.mark {
    background: var(--surface);
    border: 1.5px solid var(--line);
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
    font-size: 11.5px;
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

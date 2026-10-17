<script lang="ts">
  import Icon from "../lib/Icon.svelte";
  import Heatmap from "./Heatmap.svelte";
  import type { Forgejo } from "../lib/api";
  import { ago } from "../lib/time";
  import { now } from "../lib/clock.svelte";

  let { data }: { data: Forgejo } = $props();

  // Forgejo buckets contributions by timestamp; regroup them by local day.
  const counts = $derived.by(() => {
    const map = new Map<string, number>();
    for (const [timestamp, count] of data.heatmap) {
      const date = new Date(timestamp * 1000);
      const key = `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
      map.set(key, (map.get(key) ?? 0) + count);
    }
    return map;
  });

  const recentTotal = $derived.by(() => {
    const since = now.value / 1000 - 26 * 7 * 86400;
    return data.heatmap.filter(([t]) => t >= since).reduce((sum, [, c]) => sum + c, 0);
  });

  function short(repo: string) {
    return repo.includes("/") ? repo.split("/").slice(1).join("/") : repo;
  }
</script>

{#if data.heatmap.length > 0}
  <div class="activity">
    <p class="summary">
      <span class="figure n tnum">{recentTotal}</span>
      <span class="caption">次贡献 · 近半年</span>
    </p>
    <Heatmap {counts} maxWeeks={26} end={new Date(now.value)} label="Forgejo 近半年贡献热力图" />
  </div>
{/if}

{#if data.commits.length === 0}
  <p class="empty">还没有提交记录</p>
{:else}
  <ol class="commits">
    {#each data.commits as commit (commit.sha + commit.repo)}
      <li>
        <a class="title" href={commit.url} target="_blank" rel="noopener noreferrer">{commit.title || "(无提交信息)"}</a>
        <div class="meta">
          <a class="repo link" href={commit.repoUrl} target="_blank" rel="noopener noreferrer">{short(commit.repo)}</a>
          {#if commit.branch}<span class="branch"><Icon name="branch" size={11} />{commit.branch}</span>{/if}
          <span class="sha mono">{commit.sha.slice(0, 7)}</span>
          <span class="time tnum">{ago(commit.time, now.value)}</span>
        </div>
      </li>
    {/each}
  </ol>
{/if}

<style>
  .activity {
    margin-bottom: 16px;
  }
  .summary {
    display: flex;
    align-items: baseline;
    gap: 12px;
    margin: 0 0 12px;
  }
  .n {
    font-size: 64px;
  }
  :global(:root[data-family="panel"]) .n {
    font-size: 54px;
    color: var(--accent);
  }
  .caption {
    color: var(--muted);
    font-size: 13px;
  }
  :global(:root[data-family="panel"]) .caption {
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .commits {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li {
    display: grid;
    gap: 3px;
    padding: 9px 0;
    border-top: 1px solid var(--hair-soft);
  }
  li:first-child {
    border-top: 0;
  }
  .title {
    font-size: 13.5px;
    line-height: 1.4;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    transition: color 160ms ease;
  }
  @media (hover: hover) and (pointer: fine) {
    .title:hover {
      color: var(--accent-ink);
    }
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 2px 12px;
    font-size: 12px;
    color: var(--muted);
  }
  :global(:root[data-family="panel"]) .meta {
    font-family: var(--font-mono);
    font-size: 11.5px;
  }
  .repo {
    color: var(--ink-2);
    font-weight: 500;
  }
  .branch {
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }
  .sha {
    font-size: 11.5px;
  }
  :global(:root[data-family="panel"]) .sha,
  :global(:root[data-theme="swiss"]) .sha {
    color: var(--accent);
  }
  .empty {
    color: var(--muted);
    font-size: 13px;
  }
</style>

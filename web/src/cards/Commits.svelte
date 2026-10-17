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
    <div class="summary">
      <span class="count tnum">{recentTotal}</span>
      <span class="caption">次贡献 · 近半年</span>
    </div>
    <Heatmap {counts} maxWeeks={26} end={new Date(now.value)} label="Forgejo 近半年贡献热力图" />
  </div>
{/if}

{#if data.commits.length === 0}
  <p class="empty">还没有提交记录</p>
{:else}
  <ol class="commits">
    {#each data.commits as commit (commit.sha + commit.repo)}
      <li>
        <span class="rail" aria-hidden="true"><Icon name="commit" size={14} /></span>
        <div class="content">
          <a class="title" href={commit.url} target="_blank" rel="noopener noreferrer">{commit.title || "(无提交信息)"}</a>
          <div class="meta">
            <a class="repo" href={commit.repoUrl} target="_blank" rel="noopener noreferrer">{short(commit.repo)}</a>
            {#if commit.branch}<span class="branch"><Icon name="branch" size={11} />{commit.branch}</span>{/if}
            <span class="sha mono">{commit.sha.slice(0, 7)}</span>
            <span class="time tnum">{ago(commit.time, now.value)}</span>
          </div>
        </div>
      </li>
    {/each}
  </ol>
{/if}

<style>
  .activity {
    margin-bottom: 14px;
    padding-bottom: 14px;
    border-bottom: 1px solid var(--line);
  }
  .summary {
    display: flex;
    align-items: baseline;
    gap: 10px;
    margin: 2px 0 10px;
  }
  .count {
    font-size: 34px;
    font-weight: 600;
    letter-spacing: -0.04em;
    line-height: 1;
  }
  .caption {
    color: var(--muted);
    font-size: 13px;
  }

  .commits {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li {
    position: relative;
    display: flex;
    gap: 10px;
    padding: 6px 0;
  }
  /* A thin timeline connecting the commit dots. */
  li:not(:last-child)::before {
    content: "";
    position: absolute;
    left: 8.5px;
    top: 26px;
    bottom: -8px;
    width: 1px;
    background: var(--line-strong);
  }
  .rail {
    flex: none;
    display: grid;
    place-items: center;
    width: 18px;
    height: 20px;
    color: var(--faint);
  }
  .content {
    display: grid;
    gap: 2px;
    min-width: 0;
  }
  .title {
    font-size: 13.5px;
    line-height: 1.4;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  @media (hover: hover) and (pointer: fine) {
    .title:hover,
    .repo:hover {
      color: var(--accent-ink);
    }
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 2px 10px;
    font-size: 12px;
    color: var(--muted);
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
  .empty {
    color: var(--muted);
    font-size: 13px;
  }
</style>

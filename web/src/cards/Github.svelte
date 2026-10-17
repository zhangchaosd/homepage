<script lang="ts">
  import Icon from "../lib/Icon.svelte";
  import Heatmap from "./Heatmap.svelte";
  import type { Github } from "../lib/api";
  import { ago } from "../lib/time";
  import { now } from "../lib/clock.svelte";

  let { data }: { data: Github } = $props();

  const counts = $derived(new Map(data.contributions?.days ?? []));
  const compact = new Intl.NumberFormat("zh-CN", { notation: "compact" });

  // A small, calm palette for language dots; unknown languages fall back to grey.
  const languageColors: Record<string, string> = {
    Rust: "#c46a3f",
    TypeScript: "#3a74b8",
    JavaScript: "#c9a227",
    Svelte: "#d9572f",
    Python: "#3f72a4",
    Go: "#2a9bb8",
    Swift: "#e0603a",
    "C++": "#b0477a",
    C: "#6d7178",
    Shell: "#6a9a4a",
    HTML: "#d0613f",
    CSS: "#6a4fa0",
    Kotlin: "#9a6ae0",
    Java: "#b07219",
    Lua: "#3c3ca8",
  };
</script>

<div class="layout">
  <div class="profile">
    <a class="who" href={data.user.url} target="_blank" rel="noopener noreferrer">
      {#if data.user.avatar}
        <img class="avatar" src={data.user.avatar} alt="" width="56" height="56" />
      {:else}
        <span class="avatar fallback"><Icon name="github" size={22} /></span>
      {/if}
      <span class="names">
        <span class="name">{data.user.name ?? data.user.login}</span>
        <span class="login mono">@{data.user.login}</span>
      </span>
    </a>
    {#if data.user.bio}<p class="bio">{data.user.bio}</p>{/if}
    <dl class="stats">
      <div><dd class="figure tnum">{compact.format(data.user.publicRepos)}</dd><dt class="label">仓库</dt></div>
      <div><dd class="figure tnum">{compact.format(data.user.followers)}</dd><dt class="label">关注者</dt></div>
      <div><dd class="figure tnum">{compact.format(data.user.following)}</dd><dt class="label">关注</dt></div>
      {#if data.contributions}
        <div><dd class="figure tnum">{compact.format(data.contributions.total)}</dd><dt class="label">一年贡献</dt></div>
      {/if}
    </dl>
  </div>

  <ul class="repos">
    {#each data.repos as repo (repo.name)}
      <li>
        <a class="repo" href={repo.url} target="_blank" rel="noopener noreferrer">
          <span class="repo-name">{repo.name}</span>
          <span class="desc">{repo.description ?? "暂无描述"}</span>
          <span class="meta">
            {#if repo.language}<span class="lang"><i style:background={languageColors[repo.language] ?? "var(--faint)"}></i>{repo.language}</span>{/if}
            {#if repo.stars > 0}<span class="num tnum"><Icon name="star" size={11} />{compact.format(repo.stars)}</span>{/if}
            {#if repo.forks > 0}<span class="num tnum"><Icon name="fork" size={11} />{compact.format(repo.forks)}</span>{/if}
            <span class="pushed">{ago(repo.pushedAt, now.value)}</span>
          </span>
        </a>
      </li>
    {/each}
  </ul>
</div>

{#if data.contributions}
  <div class="calendar">
    <Heatmap {counts} maxWeeks={53} end={new Date(now.value)} label="GitHub 过去一年贡献热力图" green fill />
  </div>
{/if}

<style>
  .layout {
    display: grid;
    grid-template-columns: minmax(220px, 300px) 1fr;
    gap: 40px;
  }
  @media (max-width: 760px) {
    .layout {
      grid-template-columns: 1fr;
      gap: 20px;
    }
  }
  .who {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .avatar {
    width: 56px;
    height: 56px;
    border-radius: 50%;
    border: 1px solid var(--hair);
    object-fit: cover;
  }
  :global(:root[data-theme="panel"]) .avatar,
  :global(:root[data-theme="swiss"]) .avatar {
    border-radius: var(--tile-radius);
  }
  .avatar.fallback {
    display: grid;
    place-items: center;
    background: var(--surface-2);
  }
  .names {
    display: grid;
    gap: 3px;
  }
  .name {
    font-family: var(--font-heading);
    font-weight: var(--w-heading);
    font-size: 22px;
    letter-spacing: var(--ls-heading);
    line-height: 1.1;
  }
  :global(:root[data-theme="panel"]) .name {
    font-family: var(--font-sans);
    font-weight: 600;
    font-size: 16px;
    letter-spacing: -0.01em;
  }
  .login {
    font-size: 12px;
    color: var(--muted);
  }
  .bio {
    margin: 12px 0 0;
    font-size: 13px;
    color: var(--ink-2);
    line-height: 1.55;
  }

  /* Figures in a ruled strip. */
  .stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(70px, 1fr));
    margin: 18px 0 0;
    border-top: 1px solid var(--hair);
  }
  :global(:root[data-theme="panel"]) .stats,
  :global(:root[data-theme="swiss"]) .stats {
    gap: var(--tile-gap);
    background: var(--tile-grid-bg);
    border: 1px solid var(--tile-border);
  }
  :global(:root[data-theme="panel"]) .stats {
    border: 0;
    background: transparent;
  }
  :global(:root[data-theme="swiss"]) .stats {
    border-width: 1.5px;
  }
  .stats div {
    display: grid;
    gap: 6px;
    padding: 12px 12px 10px 0;
    border-right: 1px solid var(--hair);
  }
  .stats div:last-child {
    border-right: 0;
  }
  .stats div + div {
    padding-left: 12px;
  }
  :global(:root[data-theme="panel"]) .stats div,
  :global(:root[data-theme="swiss"]) .stats div {
    padding: 10px 12px;
    border-right: 0;
    background: var(--tile-bg);
  }
  :global(:root[data-theme="panel"]) .stats div {
    border: 1px solid var(--tile-border);
    border-radius: var(--tile-radius);
  }
  dd {
    margin: 0;
    font-size: 28px;
  }
  :global(:root[data-theme="panel"]) dd {
    font-size: 24px;
  }

  .repos {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: var(--tile-gap);
    align-content: start;
    background: var(--tile-grid-bg);
    border: 1px solid var(--tile-border);
  }
  :global(:root[data-theme="panel"]) .repos {
    border: 0;
    background: transparent;
  }
  :global(:root[data-theme="swiss"]) .repos {
    border-width: 1.5px;
  }
  .repo {
    display: grid;
    gap: 4px;
    height: 100%;
    min-height: 104px;
    padding: 13px 15px;
    background: var(--tile-bg);
    border-radius: var(--tile-radius);
    transition: background-color 160ms ease;
  }
  :global(:root[data-theme="panel"]) .repo {
    border: 1px solid var(--tile-border);
  }
  @media (hover: hover) and (pointer: fine) {
    .repo:hover {
      background: var(--surface-hover);
    }
  }
  .repo-name {
    font-weight: 580;
    font-size: 13.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .desc {
    font-size: 12.5px;
    color: var(--muted);
    line-height: 1.45;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 12px;
    margin-top: auto;
    padding-top: 6px;
    font-size: 11.5px;
    color: var(--muted);
  }
  :global(:root[data-theme="panel"]) .meta,
  :global(:root[data-theme="swiss"]) .meta {
    font-family: var(--font-mono);
    font-size: 11px;
  }
  .lang,
  .num {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  .lang i {
    width: 7px;
    height: 7px;
    border-radius: 50%;
  }
  .pushed {
    margin-left: auto;
  }
  .calendar {
    margin-top: 20px;
    padding-top: 16px;
    border-top: 1px solid var(--hair-soft);
  }
</style>

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
        <img class="avatar" src={data.user.avatar} alt="" width="52" height="52" />
      {:else}
        <span class="avatar fallback"><Icon name="github" size={22} /></span>
      {/if}
      <span class="names">
        <span class="name">{data.user.name ?? data.user.login}</span>
        <span class="login">@{data.user.login}</span>
      </span>
    </a>
    {#if data.user.bio}<p class="bio">{data.user.bio}</p>{/if}
    <dl class="stats">
      <div><dt>仓库</dt><dd class="tnum">{compact.format(data.user.publicRepos)}</dd></div>
      <div><dt>关注者</dt><dd class="tnum">{compact.format(data.user.followers)}</dd></div>
      <div><dt>关注</dt><dd class="tnum">{compact.format(data.user.following)}</dd></div>
      {#if data.contributions}
        <div><dt>今年贡献</dt><dd class="tnum">{compact.format(data.contributions.total)}</dd></div>
      {/if}
    </dl>
  </div>

  <ul class="repos">
    {#each data.repos as repo (repo.name)}
      <li>
        <a class="repo pressable" href={repo.url} target="_blank" rel="noopener noreferrer">
          <span class="repo-name">{repo.name}</span>
          <span class="desc">{repo.description ?? "暂无描述"}</span>
          <span class="meta">
            {#if repo.language}
              <span class="lang"><i style:background={languageColors[repo.language] ?? "var(--faint)"}></i>{repo.language}</span>
            {/if}
            {#if repo.stars > 0}<span class="num tnum"><Icon name="star" size={12} />{compact.format(repo.stars)}</span>{/if}
            {#if repo.forks > 0}<span class="num tnum"><Icon name="fork" size={12} />{compact.format(repo.forks)}</span>{/if}
            <span class="pushed">{ago(repo.pushedAt, now.value)}</span>
          </span>
        </a>
      </li>
    {/each}
  </ul>
</div>

{#if data.contributions}
  <div class="calendar">
    <Heatmap {counts} maxWeeks={53} end={new Date(now.value)} label="GitHub 过去一年贡献热力图" />
  </div>
{/if}

<style>
  .layout {
    display: grid;
    grid-template-columns: minmax(220px, 280px) 1fr;
    gap: 24px;
  }
  @media (max-width: 760px) {
    .layout {
      grid-template-columns: 1fr;
      gap: 16px;
    }
  }

  .who {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .avatar {
    width: 52px;
    height: 52px;
    border-radius: 50%;
    border: 1px solid var(--line);
    object-fit: cover;
  }
  .avatar.fallback {
    display: grid;
    place-items: center;
    background: var(--surface-2);
  }
  .names {
    display: grid;
  }
  .name {
    font-size: 16px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .login {
    font-size: 13px;
    color: var(--muted);
  }
  .bio {
    margin: 12px 0 0;
    font-size: 13px;
    color: var(--ink-2);
    line-height: 1.55;
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 10px 16px;
    margin: 16px 0 0;
  }
  .stats div {
    display: grid;
  }
  dt {
    font-size: 11.5px;
    color: var(--muted);
  }
  dd {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    letter-spacing: -0.02em;
  }

  .repos {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
    gap: 8px;
    align-content: start;
  }
  .repo {
    display: grid;
    gap: 4px;
    height: 100%;
    padding: 11px 13px;
    border-radius: var(--radius-tile);
    border: 1px solid var(--line);
    background: var(--surface-2);
  }
  @media (hover: hover) and (pointer: fine) {
    .repo:hover {
      background: var(--surface-hover);
      border-color: var(--line-strong);
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
    margin-top: 2px;
    font-size: 12px;
    color: var(--muted);
  }
  .lang,
  .num {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  .lang i {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }
  .pushed {
    margin-left: auto;
  }

  .calendar {
    margin-top: 18px;
    padding-top: 16px;
    border-top: 1px solid var(--line);
  }
</style>

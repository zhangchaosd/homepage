<script lang="ts">
  import type { Mail } from "../lib/api";
  import { ago } from "../lib/time";
  import { now } from "../lib/clock.svelte";

  let { data }: { data: Mail } = $props();

  // Gmail can open a message by its Message-ID; other webmails get the mailbox.
  const gmail = $derived(!!data.link && /mail\.google\.com/.test(data.link));
  function href(messageId: string | null): string | null {
    if (!data.link) return null;
    if (gmail && messageId) return `${data.link}/#search/rfc822msgid:${encodeURIComponent(messageId)}`;
    return data.link;
  }
  function initial(name: string): string {
    return Array.from(name.replace(/^["']+/, "").trim())[0]?.toUpperCase() ?? "@";
  }
</script>

<p class="summary">
  <span class="figure n tnum">{data.unread}</span>
  <span class="caption">封未读 · {data.mailbox} 共 {data.total} 封</span>
</p>

{#if data.messages.length === 0}
  <p class="empty">收件箱是空的</p>
{:else}
  <ul>
    {#each data.messages as message (message.uid)}
      {@const link = href(message.messageId)}
      <li class:unread={message.unread}>
        <svelte:element this={link ? "a" : "div"} class="item" href={link} target={link ? "_blank" : undefined} rel={link ? "noopener noreferrer" : undefined}>
          <span class="avatar figure">{initial(message.from)}</span>
          <span class="text">
            <span class="top">
              <span class="from" title={message.address}>{message.from}</span>
              <span class="time tnum">{ago(message.date, now.value)}</span>
            </span>
            <span class="subject">{message.subject || "（无主题）"}</span>
          </span>
        </svelte:element>
      </li>
    {/each}
  </ul>
{/if}

<style>
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
    align-items: flex-start;
    gap: 12px;
    padding: 8px 0;
    transition: padding-left 220ms var(--ease-out), color 160ms ease;
  }
  @media (hover: hover) and (pointer: fine) {
    a.item:hover {
      padding-left: 4px;
      color: var(--accent-ink);
    }
  }
  /* A monogram of the sender; paper sets it in italic serif like the bookmarks. */
  .avatar {
    flex: none;
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    margin-top: 1px;
    border: 1px solid var(--hair);
    border-radius: 50%;
    font-size: 15px;
    font-style: italic;
    color: var(--ink-2);
  }
  :global(:root[data-family="panel"]) .avatar,
  :global(:root[data-theme="swiss"]) .avatar {
    font-style: normal;
    font-size: 13px;
    font-weight: 700;
    border-radius: var(--tile-radius);
    background: var(--surface-2);
  }
  .text {
    display: grid;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }
  .top {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 10px;
  }
  .from {
    font-size: 13.5px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .time {
    flex: none;
    font-size: 12px;
    color: var(--muted);
  }
  :global(:root[data-family="panel"]) .time {
    font-family: var(--font-mono);
    font-size: 11.5px;
  }
  .subject {
    font-size: 13px;
    color: var(--ink-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* Unread: the sender in ink with a dot in the margin, the subject darker. */
  li.unread .from {
    font-weight: 620;
    color: var(--ink);
  }
  li.unread .subject {
    color: var(--ink);
  }
  li.unread .avatar {
    border-color: var(--accent);
    box-shadow: inset 0 0 0 1px var(--accent);
  }
  .empty {
    color: var(--muted);
    font-size: 13px;
  }
</style>

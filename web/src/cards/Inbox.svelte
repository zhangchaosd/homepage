<script lang="ts">
  import Icon from "../lib/Icon.svelte";
  import InboxDialog from "./InboxDialog.svelte";
  import { restoreInboxItem, sendToInbox, type InboxItem, type InboxStatus } from "../lib/api";
  import { ago } from "../lib/time";
  import { now } from "../lib/clock.svelte";

  let { status, link, onsent }: { status: InboxStatus | null; link?: string; onsent?: () => void } = $props();

  let title = $state("");
  let text = $state("");
  let sending = $state(false);
  let error = $state("");
  let sent = $state(false);
  let sentTimer: ReturnType<typeof setTimeout> | undefined;
  let area: HTMLTextAreaElement | undefined = $state();

  // The entry open in the sheet, and the last one moved to the trash (for undo).
  let opened = $state<InboxItem | null>(null);
  let trashed = $state<{ id: string; version: number; title: string } | null>(null);
  let undoing = $state(false);
  let trashTimer: ReturnType<typeof setTimeout> | undefined;
  const recent = $derived(status?.recent?.filter((item) => item.id !== trashed?.id) ?? null);

  function onTrashed(id: string, version: number, title: string) {
    trashed = { id, version, title };
    error = "";
    sent = false;
    setTimeout(() => onsent?.(), 700);
    clearTimeout(trashTimer);
    trashTimer = setTimeout(() => (trashed = null), 12000);
  }

  async function undo() {
    if (!trashed || undoing) return;
    undoing = true;
    try {
      await restoreInboxItem(trashed.id, trashed.version);
      trashed = null;
      setTimeout(() => onsent?.(), 500);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "撤销失败";
      trashed = null;
    } finally {
      undoing = false;
    }
  }

  const kindIcon: Record<InboxItem["kind"], string> = { text: "text", link: "globe", image: "image", file: "file", mixed: "inbox" };

  // Mirrors the server's link detection closely enough for a hint.
  const linkCount = $derived(
    new Set((text.match(/https?:\/\/[^\s<"，。；：！？、（）【】「」『』《》]+/gi) ?? []).map((url) => url.replace(/[.,;:!?)\]}>"']+$/, ""))).size,
  );
  const canSubmit = $derived(!sending && text.trim().length > 0 && status?.canSend !== false);

  const expiry = $derived.by(() => {
    if (!status?.expiresAt) return null;
    const days = Math.floor((Date.parse(status.expiresAt) - now.value) / 86_400_000);
    return Number.isFinite(days) ? days : null;
  });

  /** Hours until an item moves to the trash, when that is soon. */
  function expiringIn(item: InboxItem): number | null {
    if (!item.expiresAt) return null;
    const hours = Math.round((Date.parse(item.expiresAt) - now.value) / 3_600_000);
    return hours <= 24 ? Math.max(hours, 0) : null;
  }

  async function submit(event?: SubmitEvent) {
    event?.preventDefault();
    if (!canSubmit) return;
    sending = true;
    error = "";
    sent = false;
    try {
      await sendToInbox(text, title.trim() || null);
      title = "";
      text = "";
      sent = true;
      // The server refreshes the inbox right after a send; pick that up shortly.
      setTimeout(() => onsent?.(), 700);
      clearTimeout(sentTimer);
      sentTimer = setTimeout(() => (sent = false), 4000);
      area?.focus();
    } catch (cause) {
      // The draft stays so nothing typed is lost.
      error = cause instanceof Error ? cause.message : "投递失败";
    } finally {
      sending = false;
    }
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
      event.preventDefault();
      void submit();
    }
  }
</script>

<form class="compose" onsubmit={submit}>
  <input class="title" bind:value={title} placeholder="标题（可选）" maxlength="256" autocomplete="off" aria-label="标题" />
  <textarea bind:this={area} bind:value={text} onkeydown={onKeydown} placeholder="文字或链接，发到收件箱，在任何设备上都能看到" rows="4" aria-label="内容"></textarea>
  <div class="bar">
    <span class="hint label">
      {#if linkCount > 0}<span class="chip">{linkCount} 个链接</span>{/if}
      <span class="keys"><kbd>⌘</kbd><kbd>↵</kbd> 发送</span>
    </span>
    <button class="send pressable" type="submit" disabled={!canSubmit}>
      {sending ? "投递中…" : "投递"}<Icon name="arrow-up-right" size={13} />
    </button>
  </div>
</form>

{#if opened}
  <InboxDialog
    item={opened}
    {link}
    canManage={!!status?.canManage}
    onclose={() => (opened = null)}
    onchanged={() => setTimeout(() => onsent?.(), 500)}
    ontrashed={onTrashed}
  />
{/if}

<p class="status" role="status" aria-live="polite">
  {#if trashed}
    <span class="trashed">已移入回收站：<span class="what">{trashed.title}</span><button class="link undo" onclick={undo} disabled={undoing}>{undoing ? "撤销中…" : "撤销"}</button></span>
  {:else if error}
    <span class="error">{error}</span>
  {:else if sent}
    <span class="ok"><Icon name="check" size={12} stroke={2.4} />已投递{#if link}<a class="link" href={link} target="_blank" rel="noopener noreferrer">打开收件箱</a>{/if}</span>
  {:else if status && !status.canSend}
    <span class="error">这个令牌没有投递权限，请在收件箱设置中创建「读写」令牌</span>
  {:else if expiry !== null && expiry < 30}
    <span class="warn">令牌{expiry < 0 ? "已过期" : `将在 ${expiry} 天后过期`}，请在收件箱设置中换一个新的</span>
  {/if}
</p>

{#if recent}
  <section class="recent" aria-label="最近收件">
    <h3 class="label">最近收件</h3>
    {#if recent.length === 0}
      <p class="empty">收件箱是空的</p>
    {:else}
      <ul>
        {#each recent as item (item.id)}
          {@const soon = expiringIn(item)}
          <li>
            <button class="item" onclick={() => (opened = item)} aria-haspopup="dialog">
              <span class="kind"><Icon name={kindIcon[item.kind] ?? "inbox"} size={13} /></span>
              <span class="text">
                <span class="item-title">{item.title || item.summary || "（无标题）"}</span>
                <span class="meta">
                  <span class="tnum">{ago(item.createdAt, now.value)}</span>
                  {#if item.pinned}<span class="tag"><Icon name="pin" size={10} />置顶</span>{/if}
                  {#if item.attachments > 0}<span class="tag"><Icon name="paperclip" size={10} />{item.attachments}</span>{/if}
                  {#if soon !== null}<span class="tag soon">{soon === 0 ? "即将移入回收站" : `${soon} 小时后移入回收站`}</span>{/if}
                  {#if item.source}<span class="source">{item.source}</span>{/if}
                </span>
              </span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
{/if}

<style>
  .compose {
    display: grid;
    grid-template-rows: auto minmax(90px, auto) auto;
    border: 1px solid var(--input-border);
    border-radius: var(--input-radius);
    background: var(--input-bg);
    transition: border-color 160ms ease;
  }
  :global(:root[data-theme="swiss"]) .compose {
    border-width: 1.5px;
  }
  .compose:focus-within {
    border-color: var(--ink);
  }
  :global(:root[data-theme="panel"]) .compose:focus-within {
    border-color: var(--accent);
  }
  .title,
  textarea {
    width: 100%;
    border: 0;
    background: transparent;
    outline: none;
    font-size: 16px; /* prevents iOS zoom on focus */
    padding: 10px 12px;
  }
  @media (hover: hover) and (pointer: fine) {
    .title,
    textarea {
      font-size: 13.5px;
    }
  }
  .title {
    height: 38px;
    border-bottom: 1px solid var(--hair-soft);
    font-weight: 550;
  }
  textarea {
    min-height: 96px;
    line-height: 1.5;
    resize: vertical;
  }
  :global(:root[data-theme="panel"]) textarea,
  :global(:root[data-theme="panel"]) .title {
    font-family: var(--font-mono);
  }
  .title::placeholder,
  textarea::placeholder {
    color: var(--muted);
  }

  .bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 6px 6px 6px 12px;
    border-top: 1px solid var(--hair-soft);
  }
  .hint {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .keys {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  @media (hover: none) {
    .keys {
      display: none;
    }
  }
  kbd {
    display: inline-grid;
    place-items: center;
    min-width: 16px;
    height: 16px;
    padding: 0 3px;
    border: 1px solid var(--hair);
    border-radius: var(--chip-radius);
    font: 10px var(--font-mono);
  }
  .chip {
    color: var(--accent);
  }
  .send {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 32px;
    padding: 0 14px;
    background: var(--btn-bg);
    color: var(--btn-fg);
    border-radius: var(--btn-radius);
    font-size: 13px;
    font-weight: 550;
  }
  .send:disabled {
    opacity: 0.4;
    cursor: default;
    transform: none;
  }

  .status {
    min-height: 18px;
    margin: 10px 0 0;
    font-size: 12.5px;
  }
  .ok {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--ok);
    font-weight: 500;
  }
  .ok a {
    margin-left: 6px;
    color: var(--muted);
    font-weight: 400;
  }
  .warn {
    color: var(--warn);
  }
  .trashed {
    display: inline-flex;
    align-items: baseline;
    gap: 2px;
    max-width: 100%;
    color: var(--muted);
  }
  .trashed .what {
    max-width: 16em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--ink-2);
  }
  .undo {
    margin-left: 10px;
    color: var(--accent-ink, var(--accent));
    font-weight: 550;
  }
  .error {
    color: var(--danger);
  }

  .recent {
    margin-top: 14px;
  }
  h3 {
    margin: 0 0 4px;
  }
  .recent ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .recent li {
    border-top: 1px solid var(--hair-soft);
  }
  .recent li:first-child {
    border-top: 0;
  }
  .item {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    width: 100%;
    padding: 8px 0;
    text-align: left;
    transition: padding-left 220ms var(--ease-out), color 160ms ease;
  }
  @media (hover: hover) and (pointer: fine) {
    .item:hover {
      padding-left: 4px;
      color: var(--accent-ink);
    }
  }
  .kind {
    flex: none;
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    margin-top: 1px;
    color: var(--muted);
  }
  .text {
    display: grid;
    gap: 1px;
    min-width: 0;
  }
  .item-title {
    font-size: 13.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 2px 8px;
    font-size: 11.5px;
    color: var(--muted);
  }
  :global(:root[data-theme="panel"]) .meta {
    font-family: var(--font-mono);
  }
  .tag {
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }
  .tag.soon {
    color: var(--warn);
  }
  .source {
    color: var(--faint);
  }
  .empty {
    margin: 4px 0 0;
    font-size: 13px;
    color: var(--muted);
  }
</style>

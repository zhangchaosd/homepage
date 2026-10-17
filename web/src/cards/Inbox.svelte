<script lang="ts">
  import Icon from "../lib/Icon.svelte";
  import { sendToInbox, type InboxStatus } from "../lib/api";
  import { now } from "../lib/clock.svelte";

  let { status, link }: { status: InboxStatus | null; link?: string } = $props();

  let title = $state("");
  let text = $state("");
  let sending = $state(false);
  let error = $state("");
  let sent = $state(false);
  let sentTimer: ReturnType<typeof setTimeout> | undefined;
  let area: HTMLTextAreaElement | undefined = $state();

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
  <textarea
    bind:this={area}
    bind:value={text}
    onkeydown={onKeydown}
    placeholder="文字或链接，发到收件箱，在任何设备上都能看到"
    rows="4"
    aria-label="内容"
  ></textarea>
  <div class="bar">
    <span class="hint">
      {#if linkCount > 0}
        <span class="chip"><Icon name="globe" size={12} />{linkCount} 个链接</span>
      {/if}
      <span class="keys"><kbd>⌘</kbd><kbd>↵</kbd> 发送</span>
    </span>
    <button class="send pressable" type="submit" disabled={!canSubmit}>
      <Icon name="send" size={14} />
      {sending ? "投递中…" : "投递"}
    </button>
  </div>
</form>

<p class="status" role="status" aria-live="polite">
  {#if error}
    <span class="error">{error}</span>
  {:else if sent}
    <span class="ok"><Icon name="check" size={13} stroke={2.4} />已投递{#if link}<a href={link} target="_blank" rel="noopener noreferrer">打开收件箱</a>{/if}</span>
  {:else if status && !status.canSend}
    <span class="error">这个令牌没有投递权限，请在收件箱设置中创建只写令牌</span>
  {:else if expiry !== null && expiry < 30}
    <span class="warn">令牌{expiry < 0 ? "已过期" : `将在 ${expiry} 天后过期`}，请在收件箱设置中换一个新的</span>
  {/if}
</p>

<style>
  /* The text area grows to fill the card when a neighbour makes the row tall. */
  .compose {
    flex: 1;
    display: grid;
    grid-template-rows: auto minmax(96px, 1fr) auto;
    gap: 8px;
  }
  .title,
  textarea {
    width: 100%;
    border: 1px solid var(--line-strong);
    border-radius: 11px;
    background: var(--surface-2);
    font-size: 16px; /* prevents iOS zoom on focus */
    outline: none;
    transition:
      border-color 160ms ease,
      box-shadow 160ms ease;
  }
  @media (hover: hover) and (pointer: fine) {
    .title,
    textarea {
      font-size: 13.5px;
    }
  }
  .title {
    height: 36px;
    padding: 0 12px;
  }
  textarea {
    height: 100%;
    min-height: 96px;
    max-height: 420px;
    padding: 10px 12px;
    line-height: 1.5;
    resize: vertical;
  }
  .title:focus,
  textarea:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
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
  }
  .hint {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--muted);
  }
  .keys {
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }
  @media (hover: none) {
    .keys {
      display: none;
    }
  }
  kbd {
    display: inline-grid;
    place-items: center;
    min-width: 18px;
    height: 18px;
    padding: 0 4px;
    border-radius: 5px;
    border: 1px solid var(--line-strong);
    background: var(--surface-2);
    font-family: var(--font-mono);
    font-size: 10.5px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin-right: 6px;
    padding: 1px 8px 1px 6px;
    border-radius: 999px;
    background: var(--accent-soft);
    color: var(--accent-ink);
    font-weight: 500;
  }

  .send {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 34px;
    padding: 0 14px;
    border-radius: 10px;
    background: var(--ink);
    color: var(--surface);
    font-size: 13px;
    font-weight: 550;
  }
  .send:disabled {
    opacity: 0.45;
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
    gap: 5px;
    color: var(--accent-ink);
    font-weight: 500;
  }
  .ok a {
    margin-left: 6px;
    color: var(--muted);
    font-weight: 400;
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .warn {
    color: var(--warn);
  }
  .error {
    color: var(--danger);
  }
</style>

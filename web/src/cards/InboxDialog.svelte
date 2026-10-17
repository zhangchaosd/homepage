<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "../lib/Icon.svelte";
  import Sheet from "../lib/Sheet.svelte";
  import {
    getInboxItem,
    inboxAttachmentUrl,
    trashInboxItem,
    updateInboxItem,
    type InboxDetail,
    type InboxItem,
  } from "../lib/api";
  import { ago } from "../lib/time";
  import { now } from "../lib/clock.svelte";

  /** One inbox entry: full text, links and attachments, with rename, pin,
   *  copy and trash. Without a token that may manage items it shows the
   *  summary and points to the inbox. */
  let {
    item,
    link,
    canManage,
    onclose,
    onchanged,
    ontrashed,
  }: {
    item: InboxItem;
    link?: string;
    canManage: boolean;
    onclose: () => void;
    onchanged: () => void;
    ontrashed: (id: string, version: number, title: string) => void;
  } = $props();

  let sheet: Sheet | undefined = $state();
  let detail = $state<InboxDetail | null>(null);
  let loading = $state(false);
  let error = $state("");
  let busy = $state<"" | "title" | "pin" | "trash">("");
  let copied = $state("");
  let draft = $state("");

  const stamp = new Intl.DateTimeFormat("zh-CN", { month: "long", day: "numeric", weekday: "short", hour: "2-digit", minute: "2-digit", hour12: false });
  const heading = $derived(detail ? detail.title || "（无标题）" : item.title || item.summary || "（无标题）");
  const images = $derived(detail?.attachments.filter((file) => file.preview) ?? []);
  const files = $derived(detail?.attachments.filter((file) => !file.preview) ?? []);

  const expiry = $derived.by(() => {
    const at = detail?.expiresAt ?? item.expiresAt;
    if (!at) return "长期保留";
    const hours = Math.round((Date.parse(at) - now.value) / 3_600_000);
    if (hours <= 0) return "即将移入回收站";
    return hours < 48 ? `${hours} 小时后移入回收站` : `${Math.round(hours / 24)} 天后移入回收站`;
  });

  onMount(async () => {
    if (!canManage) return;
    loading = true;
    try {
      detail = await getInboxItem(item.id);
      draft = detail.title;
    } catch (cause) {
      error = message(cause);
    } finally {
      loading = false;
    }
  });

  function message(cause: unknown) {
    return cause instanceof Error ? cause.message : "操作失败";
  }

  function size(bytes: number) {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(bytes < 10240 ? 1 : 0)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  }

  function host(url: string) {
    try {
      return new URL(url).host.replace(/^www\./, "");
    } catch {
      return url;
    }
  }

  async function change(kind: "title" | "pin", patch: { title?: string; pinned?: boolean }) {
    if (!detail || busy) return;
    busy = kind;
    error = "";
    try {
      detail = await updateInboxItem(detail.id, detail.version, patch);
      draft = detail.title;
      onchanged();
    } catch (cause) {
      error = message(cause);
      if (detail) draft = detail.title;
    } finally {
      busy = "";
    }
  }

  function saveTitle() {
    if (detail && draft.trim() !== detail.title) void change("title", { title: draft.trim() });
  }

  function onTitleKey(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();
      (event.currentTarget as HTMLInputElement).blur();
    } else if (event.key === "Escape" && detail && draft !== detail.title) {
      // First Escape undoes the edit; the next one closes the sheet.
      event.preventDefault();
      draft = detail.title;
    }
  }

  async function copy(text: string, what: string) {
    try {
      await navigator.clipboard.writeText(text);
      copied = what;
      setTimeout(() => (copied = ""), 1600);
    } catch {
      error = "无法写入剪贴板";
    }
  }

  async function trash() {
    if (!detail || busy) return;
    busy = "trash";
    error = "";
    try {
      const { version } = await trashInboxItem(detail.id, detail.version);
      busy = "";
      ontrashed(detail.id, version, heading);
      sheet?.close();
    } catch (cause) {
      error = message(cause);
      busy = "";
    }
  }
</script>

<Sheet bind:this={sheet} label={heading} busy={busy === "trash"} width={600} {onclose}>
  {#snippet title()}
    {#if detail}
      <input
        class="title-input"
        bind:value={draft}
        onblur={saveTitle}
        onkeydown={onTitleKey}
        placeholder="（无标题）"
        maxlength="256"
        aria-label="标题，回车保存"
        disabled={busy === "title"}
      />
    {:else}
      <h2>{heading}</h2>
    {/if}
    <p class="meta label tnum">
      <span title={stamp.format(new Date(item.createdAt))}>{ago(item.createdAt, now.value)}</span>
      {#if (detail?.source ?? item.source)}<span>{detail?.source ?? item.source}</span>{/if}
      {#if detail?.pinned ?? item.pinned}<span class="pinned"><Icon name="pin" size={10} />置顶</span>{/if}
      <span>{expiry}</span>
    </p>
  {/snippet}

  {#if loading}
    <div class="skeleton" aria-hidden="true"><span style="width: 86%"></span><span style="width: 64%"></span><span style="width: 72%"></span></div>
  {:else if detail}
    {#if detail.text}
      <div class="body">
        <p class="text">{detail.text}</p>
        <button class="copy pressable" onclick={() => copy(detail!.text, "text")} aria-label="复制正文">
          <Icon name={copied === "text" ? "check" : "copy"} size={13} />
        </button>
      </div>
    {/if}

    {#if detail.links.length > 0}
      <section>
        <h3 class="label">链接 {detail.links.length}</h3>
        <ul class="links">
          {#each detail.links as entry (entry.url)}
            <li>
              <a href={entry.url} target="_blank" rel="noopener noreferrer" class="row">
                <Icon name="globe" size={13} />
                <span class="link-text">
                  <span class="host">{entry.title || host(entry.url)}</span>
                  <span class="url">{entry.url}</span>
                </span>
                <Icon name="arrow-up-right" size={12} />
              </a>
              <button class="mini pressable" onclick={() => copy(entry.url, entry.url)} aria-label="复制链接">
                <Icon name={copied === entry.url ? "check" : "copy"} size={12} />
              </button>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    {#if images.length > 0}
      <section>
        <h3 class="label">图片 {images.length}</h3>
        <div class="images">
          {#each images as file (file.id)}
            <a class="thumb" href={inboxAttachmentUrl(file.id)} download={file.name} title="下载 {file.name} · {size(file.size)}">
              <img src={inboxAttachmentUrl(file.id, true)} alt={file.name} loading="lazy" />
              <span class="thumb-meta"><Icon name="download" size={11} />{size(file.size)}</span>
            </a>
          {/each}
        </div>
      </section>
    {/if}

    {#if files.length > 0}
      <section>
        <h3 class="label">文件 {files.length}</h3>
        <ul class="links">
          {#each files as file (file.id)}
            <li>
              <a class="row" href={inboxAttachmentUrl(file.id)} download={file.name}>
                <Icon name="file" size={13} />
                <span class="link-text">
                  <span class="host">{file.name}</span>
                  <span class="url">{file.mediaType || "文件"} · {size(file.size)}</span>
                </span>
                <Icon name="download" size={12} />
              </a>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    {#if !detail.text && detail.links.length === 0 && detail.attachments.length === 0}
      <p class="empty">这一条没有内容</p>
    {/if}
  {:else}
    {#if item.summary}<p class="text">{item.summary}</p>{/if}
    {#if !canManage}
      <p class="notice">
        要在这里查看全文、附件，或置顶、改名、移入回收站，需要收件箱 0.4.0 起新建的<strong>读写</strong>令牌：在收件箱「设置 → 令牌」创建后，换到主页配置的 <code>[tokens] inbox</code>。
      </p>
    {/if}
  {/if}

  {#if error}<p class="error" role="alert">{error}</p>{/if}

  {#snippet footer()}
    {#if detail}
      <button class="btn pressable" onclick={() => change("pin", { pinned: !detail!.pinned })} disabled={!!busy} aria-pressed={detail.pinned}>
        <Icon name="pin" size={13} />{detail.pinned ? "取消置顶" : "置顶"}
      </button>
      <button class="btn danger pressable" onclick={trash} disabled={!!busy}>
        <Icon name="trash" size={13} />{busy === "trash" ? "移动中…" : "移入回收站"}
      </button>
    {/if}
    <span class="spacer"></span>
    {#if link}
      <a class="btn pressable" href={link} target="_blank" rel="noopener noreferrer">在收件箱打开<Icon name="arrow-up-right" size={12} /></a>
    {/if}
  {/snippet}
</Sheet>

<style>
  .title-input {
    width: 100%;
    margin: -4px 0 0 -6px;
    padding: 2px 6px;
    border: 1px solid transparent;
    border-radius: var(--input-radius);
    background: transparent;
    outline: none;
    font-family: var(--font-heading);
    font-weight: var(--w-heading);
    font-size: 22px;
    letter-spacing: var(--ls-heading);
    line-height: 1.3;
    color: var(--ink);
    transition: border-color 160ms ease;
  }
  :global(:root[data-family="panel"]) .title-input {
    font-family: var(--font-sans);
    font-size: 17px;
    font-weight: 600;
  }
  @media (hover: hover) and (pointer: fine) {
    .title-input:hover {
      border-color: var(--hair);
    }
  }
  .title-input:focus {
    border-color: var(--accent);
  }
  .title-input::placeholder {
    color: var(--faint);
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    margin: 10px 0 0;
  }
  .pinned {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    color: var(--accent-ink, var(--accent));
  }

  .body {
    position: relative;
    margin-top: 8px;
    padding: 14px 40px 14px 16px;
    background: var(--surface-2);
    border-radius: var(--tile-radius);
  }
  .text {
    margin: 0;
    font-size: 14px;
    line-height: 1.65;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .copy,
  .mini {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    color: var(--muted);
    border-radius: var(--chip-radius);
  }
  .copy {
    position: absolute;
    top: 8px;
    right: 8px;
  }
  @media (hover: hover) and (pointer: fine) {
    .copy:hover,
    .mini:hover {
      color: var(--ink);
      background: var(--surface-hover);
    }
  }

  section {
    margin-top: 20px;
  }
  h3 {
    margin: 0 0 6px;
  }
  .links {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .links li {
    display: flex;
    align-items: center;
    gap: 4px;
    border-top: 1px solid var(--hair-soft);
  }
  .links li:first-child {
    border-top: 0;
  }
  .row {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 0;
    color: var(--muted);
    transition: color 160ms ease;
  }
  @media (hover: hover) and (pointer: fine) {
    .row:hover {
      color: var(--ink);
    }
  }
  .link-text {
    flex: 1;
    min-width: 0;
    display: grid;
    gap: 1px;
  }
  .host {
    color: var(--ink);
    font-size: 13.5px;
    font-weight: 520;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .url {
    font: 11.5px var(--font-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .images {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(132px, 1fr));
    gap: 8px;
  }
  .thumb {
    position: relative;
    display: block;
    aspect-ratio: 4 / 3;
    overflow: hidden;
    border-radius: var(--tile-radius);
    background: var(--surface-2);
  }
  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    transition: transform 300ms var(--ease-out);
  }
  .thumb-meta {
    position: absolute;
    left: 6px;
    bottom: 6px;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 6px;
    border-radius: var(--chip-radius);
    background: rgba(0, 0, 0, 0.55);
    color: #fff;
    font: 10.5px var(--font-mono);
  }
  @media (hover: hover) and (pointer: fine) {
    .thumb:hover img {
      transform: scale(1.03);
    }
  }

  .skeleton {
    display: grid;
    gap: 12px;
    padding: 12px 0;
  }
  .skeleton span {
    height: 11px;
    background: linear-gradient(90deg, var(--heat-0), var(--hair), var(--heat-0));
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

  .notice {
    margin: 16px 0 0;
    padding: 12px 14px;
    border-left: 2px solid var(--warn);
    background: var(--warn-soft);
    font-size: 13px;
    line-height: 1.6;
    color: var(--ink-2);
  }
  code {
    font: 12px var(--font-mono);
  }
  .empty {
    color: var(--muted);
    font-size: 13px;
  }
  .error {
    margin: 14px 0 0;
    color: var(--danger);
    font-size: 12.5px;
  }
  .spacer {
    flex: 1;
  }
  @media (max-width: 480px) {
    .spacer {
      display: none;
    }
  }
</style>

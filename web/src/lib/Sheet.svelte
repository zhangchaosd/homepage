<script lang="ts">
  import { onMount, type Snippet } from "svelte";
  import Icon from "./Icon.svelte";

  /** A modal sheet on the native <dialog>: Escape and a click outside close it
   *  unless `busy`; focus is trapped and returned by the browser. */
  let {
    title,
    label,
    busy = false,
    width = 560,
    onclose,
    children,
    footer,
  }: {
    title?: Snippet;
    label: string;
    busy?: boolean;
    width?: number;
    onclose: () => void;
    children: Snippet;
    footer?: Snippet;
  } = $props();

  let dialog: HTMLDialogElement | undefined = $state();

  onMount(() => dialog?.showModal());

  export function close() {
    if (!busy) dialog?.close();
  }
</script>

<dialog
  bind:this={dialog}
  style:--sheet-width="{width}px"
  {onclose}
  oncancel={(event) => {
    if (busy) event.preventDefault();
  }}
  onclick={(event) => {
    if (event.target === dialog) close();
  }}
  aria-label={label}
>
  <div class="sheet">
    <header>
      <div class="title">{#if title}{@render title()}{:else}<h2>{label}</h2>{/if}</div>
      <button class="close pressable" onclick={close} aria-label="关闭" disabled={busy}><Icon name="x" size={16} /></button>
    </header>
    <div class="content">{@render children()}</div>
    {#if footer}<footer>{@render footer()}</footer>{/if}
  </div>
</dialog>

<style>
  dialog {
    width: min(calc(100% - 32px), var(--sheet-width));
    max-height: calc(100dvh - 48px);
    padding: 0;
    border: var(--card-border-w, 1px) solid var(--line);
    border-radius: var(--card-radius);
    background: var(--surface);
    color: var(--ink);
    box-shadow: 0 24px 60px -20px rgba(0, 0, 0, 0.4);
    overflow: hidden;
    opacity: 1;
    transform: none;
    transition:
      opacity 200ms var(--ease-out),
      transform 200ms var(--ease-out),
      overlay 200ms allow-discrete,
      display 200ms allow-discrete;
  }
  :global(:root[data-theme="paper"]) dialog,
  :global(:root:not([data-theme])) dialog {
    border-width: 1px;
    background: var(--bg);
  }
  @starting-style {
    dialog[open] {
      opacity: 0;
      transform: scale(0.98) translateY(6px);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    dialog {
      transition: opacity 150ms ease;
      transform: none;
    }
  }
  dialog::backdrop {
    background: rgba(10, 10, 9, 0.4);
    backdrop-filter: blur(3px);
  }

  /* Header and footer stay put; only the content scrolls. */
  .sheet {
    display: flex;
    flex-direction: column;
    max-height: calc(100dvh - 48px);
  }
  header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    padding: 20px 24px 12px;
  }
  .title {
    flex: 1;
    min-width: 0;
  }
  h2,
  .title :global(h2) {
    margin: 0;
    font-family: var(--font-heading);
    font-weight: var(--w-heading);
    font-size: 22px;
    letter-spacing: var(--ls-heading);
    line-height: 1.2;
  }
  :global(:root[data-theme="panel"]) h2 {
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.14em;
  }
  .close {
    flex: none;
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    margin: -4px -8px 0 0;
    color: var(--muted);
  }
  .content {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 4px 24px 20px;
  }
  footer {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    padding: 14px 24px 18px;
    border-top: 1px solid var(--hair);
  }
  @media (max-width: 480px) {
    header {
      padding: 16px 18px 10px;
    }
    .content {
      padding: 4px 18px 16px;
    }
    footer {
      padding: 12px 18px 16px;
    }
  }
</style>

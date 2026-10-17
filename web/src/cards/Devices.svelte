<script lang="ts">
  import Icon from "../lib/Icon.svelte";
  import type { Device } from "../lib/api";

  let { devices }: { devices: Device[] } = $props();

  let copied = $state<string>("");
  let expanded = $state<Record<string, boolean>>({});
  let timer: ReturnType<typeof setTimeout> | undefined;

  /** Icon from the OS ipv6who reports: Linux boxes are servers (ARM boards get a chip). */
  function iconFor(device: Device): string {
    switch (device.os) {
      case "linux":
        return device.arch === "aarch64" || device.arch === "arm64" ? "chip" : "server";
      case "macos":
      case "windows":
        return "desktop";
      default:
        return "server";
    }
  }

  async function copy(address: string) {
    try {
      await navigator.clipboard.writeText(address);
    } catch {
      return; // Clipboard needs a secure context; select-and-copy still works.
    }
    copied = address;
    clearTimeout(timer);
    timer = setTimeout(() => (copied = ""), 1400);
  }

  /** Breaks an address into groups so long ones wrap at colons, never mid-group. */
  function segments(address: string): string[] {
    return address.split(/(?<=:)/);
  }

  const online = $derived(devices.filter((device) => device.address).length);
</script>

<p class="summary">
  <span class="figure n tnum">{online}<small>/{devices.length}</small></span>
  <span class="caption">台设备有公网 IPv6</span>
</p>

<ul>
  {#each devices as device (device.url)}
    {@const others = device.addresses.filter((a) => a.address !== device.address)}
    <li>
      <span class="kind" class:offline={!device.address}><Icon name={iconFor(device)} size={15} /></span>
      <div class="info">
        <div class="head">
          <span class="name" title={device.url}>{device.name}</span>
          {#if device.osName}<span class="os mono">{device.osName}</span>{/if}
          {#if device.interface}<span class="os mono">{device.interface}</span>{/if}
          {#if device.addressType && device.addressType !== "static"}<span class="os mono">{device.addressType}</span>{/if}
        </div>
        {#if device.address}
          <button class="address mono pressable" class:done={copied === device.address} onclick={() => copy(device.address!)} title="点击复制">
            <span class="value">{#each segments(device.address) as part}<span class="seg">{part}</span>{/each}</span>
            <span class="copy-icon" aria-hidden="true"><Icon name={copied === device.address ? "check" : "copy"} size={12} stroke={2} /></span>
            <span class="visually-hidden" aria-live="polite">{copied === device.address ? "已复制" : ""}</span>
          </button>
          {#if others.length > 0}
            <button class="more link" onclick={() => (expanded[device.url] = !expanded[device.url])} aria-expanded={!!expanded[device.url]}>
              {expanded[device.url] ? "收起" : `另有 ${others.length} 个地址`}
            </button>
            {#if expanded[device.url]}
              <ul class="others">
                {#each others as other (other.address)}
                  <li>
                    <button class="address small mono pressable" class:done={copied === other.address} onclick={() => copy(other.address)}>
                      <span class="value">{#each segments(other.address) as part}<span class="seg">{part}</span>{/each}</span>
                      <span class="hint">{other.kind ?? ""}{other.deprecated ? " · deprecated" : ""}</span>
                    </button>
                  </li>
                {/each}
              </ul>
            {/if}
          {/if}
        {:else}
          <span class="error">{device.error ?? "无地址"}</span>
        {/if}
      </div>
    </li>
  {/each}
</ul>

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
  .n small {
    font-size: 0.5em;
    color: var(--faint);
    font-weight: inherit;
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
    display: flex;
    gap: 12px;
    padding: 11px 0;
    border-top: 1px solid var(--hair-soft);
  }
  li:first-child {
    border-top: 0;
  }

  .kind {
    flex: none;
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    margin-top: 1px;
    color: var(--ink);
  }
  :global(:root[data-family="panel"]) .kind,
  :global(:root[data-theme="swiss"]) .kind {
    background: var(--surface-2);
    border: 1px solid var(--hair);
    border-radius: var(--tile-radius);
    color: var(--ink-2);
  }
  :global(:root[data-theme="swiss"]) .kind {
    border-width: 1.5px;
    border-color: var(--line);
    background: var(--surface);
  }
  .kind.offline {
    color: var(--faint);
  }

  .info {
    display: grid;
    gap: 3px;
    min-width: 0;
    flex: 1;
  }
  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 4px 10px;
    min-width: 0;
  }
  .name {
    font-weight: 560;
    font-size: 13.5px;
  }
  .os {
    font-size: 11px;
    color: var(--muted);
  }

  .address {
    display: flex;
    align-items: center;
    gap: 8px;
    width: fit-content;
    max-width: 100%;
    margin-left: -4px;
    padding: 1px 4px;
    text-align: left;
    font-size: 13px;
    color: var(--ink);
    -webkit-user-select: text;
    user-select: text;
  }
  :global(:root[data-family="panel"]) .address {
    color: var(--data);
  }
  .address.small {
    font-size: 12px;
    color: var(--ink-2);
  }
  .value {
    display: inline;
    overflow-wrap: anywhere;
  }
  .seg {
    display: inline-block;
  }
  .copy-icon {
    color: var(--faint);
    opacity: 0;
    transition: opacity 140ms ease;
  }
  .address.done .copy-icon {
    opacity: 1;
    color: var(--ok);
  }
  @media (hover: hover) and (pointer: fine) {
    .address:hover {
      background: var(--surface-hover);
    }
    .address:hover .copy-icon {
      opacity: 1;
    }
  }
  @media (hover: none) {
    .copy-icon {
      opacity: 1;
    }
  }
  .hint {
    font-family: var(--font-sans);
    font-size: 11px;
    color: var(--muted);
    white-space: nowrap;
  }
  .more {
    width: fit-content;
    font-size: 12px;
    color: var(--muted);
  }
  .others li {
    padding: 2px 0;
    border: 0;
  }
  .error {
    font-size: 12.5px;
    color: var(--danger);
  }
  :global(:root[data-family="panel"]) .error {
    font-family: var(--font-mono);
    text-transform: uppercase;
    font-size: 11.5px;
    letter-spacing: 0.04em;
  }
</style>

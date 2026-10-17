<script lang="ts">
  import Icon from "../lib/Icon.svelte";
  import type { Device } from "../lib/api";

  let { devices }: { devices: Device[] } = $props();

  let copied = $state<string>("");
  let expanded = $state<Record<string, boolean>>({});
  let timer: ReturnType<typeof setTimeout> | undefined;

  const kinds: Record<string, string> = {
    server: "server",
    nas: "nas",
    desktop: "desktop",
    pc: "desktop",
    laptop: "laptop",
    router: "router",
    phone: "phone",
    pi: "chip",
    raspberrypi: "chip",
    sbc: "chip",
  };

  async function copy(address: string) {
    try {
      await navigator.clipboard.writeText(address);
    } catch {
      // Clipboard needs a secure context; select-and-copy still works on the text.
      return;
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

<div class="summary">
  <span class="count tnum">{online}<span class="of">/{devices.length}</span></span>
  <span class="caption">台设备有公网 IPv6</span>
</div>

<ul>
  {#each devices as device (device.name)}
    {@const others = device.addresses.filter((a) => a.address !== device.address)}
    <li>
      <span class="kind" class:offline={!device.address}>
        <Icon name={kinds[device.kind.toLowerCase()] ?? "server"} size={17} />
      </span>
      <div class="info">
        <div class="head">
          <span class="name">{device.name}</span>
          {#if device.interface}<span class="tag mono">{device.interface}</span>{/if}
          {#if device.addressType && device.addressType !== "static"}<span class="tag">{device.addressType}</span>{/if}
        </div>
        {#if device.address}
          <button
            class="address mono pressable"
            class:done={copied === device.address}
            onclick={() => copy(device.address!)}
            title="点击复制"
          >
            <span class="value">{#each segments(device.address) as part}<span class="seg">{part}</span>{/each}</span>
            <span class="copy-icon" aria-hidden="true">
              <Icon name={copied === device.address ? "check" : "copy"} size={13} stroke={2} />
            </span>
            <span class="visually-hidden" aria-live="polite">{copied === device.address ? "已复制" : ""}</span>
          </button>
          {#if others.length > 0}
            <button class="more" onclick={() => (expanded[device.name] = !expanded[device.name])} aria-expanded={!!expanded[device.name]}>
              {expanded[device.name] ? "收起" : `另有 ${others.length} 个地址`}
            </button>
            {#if expanded[device.name]}
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
    gap: 10px;
    margin: 2px 0 10px;
  }
  .count {
    font-size: 34px;
    font-weight: 600;
    letter-spacing: -0.04em;
    line-height: 1;
  }
  .of {
    color: var(--faint);
    font-weight: 500;
  }
  .caption {
    color: var(--muted);
    font-size: 13px;
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
    border-top: 1px solid var(--line);
  }
  li:first-child {
    border-top: 0;
  }

  .kind {
    flex: none;
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: 10px;
    background: var(--accent-soft);
    color: var(--accent-ink);
  }
  .kind.offline {
    background: var(--surface-2);
    color: var(--faint);
    border: 1px solid var(--line);
  }

  .info {
    display: grid;
    gap: 3px;
    min-width: 0;
    flex: 1;
  }

  .head {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .name {
    font-weight: 560;
    font-size: 13.5px;
  }
  .tag {
    font-size: 11px;
    padding: 0 6px;
    border-radius: 5px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    color: var(--muted);
    line-height: 17px;
  }

  .address {
    display: flex;
    align-items: center;
    gap: 8px;
    width: fit-content;
    max-width: 100%;
    margin-left: -6px;
    padding: 2px 6px;
    border-radius: 7px;
    text-align: left;
    font-size: 13px;
    color: var(--ink);
    -webkit-user-select: text;
    user-select: text;
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
    color: var(--accent);
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
    padding: 0;
  }
  @media (hover: hover) and (pointer: fine) {
    .more:hover {
      color: var(--ink);
    }
  }
  .others li {
    padding: 2px 0;
    border: 0;
  }

  .error {
    font-size: 12.5px;
    color: var(--danger);
  }
</style>

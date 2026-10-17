<script lang="ts">
  import Icon from "../lib/Icon.svelte";
  import type { HaEntity } from "../lib/api";

  let { entities }: { entities: HaEntity[] } = $props();

  const byClass: Record<string, string> = {
    temperature: "thermometer",
    humidity: "droplet",
    moisture: "droplet",
    power: "bolt",
    energy: "bolt",
    current: "bolt",
    voltage: "bolt",
    battery: "battery",
    door: "door",
    window: "door",
    opening: "door",
    wind_speed: "wind",
    illuminance: "bulb",
  };
  const byDomain: Record<string, string> = { light: "bulb", switch: "power", fan: "wind", climate: "thermometer", lock: "key", person: "eye", cover: "door" };

  const words: Record<string, string> = {
    on: "开",
    off: "关",
    home: "在家",
    not_home: "外出",
    open: "开",
    closed: "关",
    opening: "开启中",
    closing: "关闭中",
    locked: "已锁",
    unlocked: "未锁",
    unavailable: "不可用",
    unknown: "未知",
    heat: "制热",
    cool: "制冷",
    idle: "空闲",
  };

  function iconFor(entity: HaEntity) {
    if (entity.icon) return entity.icon;
    if (entity.deviceClass && byClass[entity.deviceClass]) return byClass[entity.deviceClass];
    return byDomain[entity.id.split(".")[0]] ?? "circle";
  }

  function display(entity: HaEntity): { value: string; unit: string; active: boolean } {
    const state = entity.state ?? "";
    const number = Number(state);
    if (state !== "" && Number.isFinite(number)) {
      const value = Math.abs(number) >= 100 ? Math.round(number).toString() : (Math.round(number * 10) / 10).toString();
      return { value, unit: entity.unit ?? "", active: true };
    }
    return { value: words[state] ?? state, unit: "", active: ["on", "open", "home", "unlocked", "heat", "cool"].includes(state) };
  }
</script>

{#if entities.length === 0}
  <p class="empty">在 config.toml 的 [homeassistant] 中添加 entities</p>
{:else}
  <ul class="grid">
    {#each entities as entity (entity.id)}
      {@const shown = display(entity)}
      <li class="cell" class:active={shown.active && !entity.error} class:failed={!!entity.error} title={entity.error ?? entity.id}>
        <span class="icon"><Icon name={iconFor(entity)} size={14} /></span>
        <span class="value figure tnum">
          {#if entity.error}—{:else}{shown.value}{#if shown.unit}<small>{shown.unit}</small>{/if}{/if}
        </span>
        <span class="name">{entity.name}</span>
      </li>
    {/each}
  </ul>
{/if}

<style>
  /* Instruments in a ruled grid, two across. */
  .grid {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--tile-gap);
    background: var(--tile-grid-bg);
    border: 1px solid var(--tile-border);
  }
  :global(:root[data-theme="panel"]) .grid {
    border: 0;
    background: transparent;
  }
  :global(:root[data-theme="swiss"]) .grid {
    border-width: 1.5px;
  }
  .cell {
    position: relative;
    display: grid;
    gap: 4px;
    padding: 14px 14px 12px;
    min-width: 0;
    background: var(--tile-bg);
    border-radius: var(--tile-radius);
  }
  :global(:root[data-theme="panel"]) .cell {
    border: 1px solid var(--tile-border);
  }
  .icon {
    position: absolute;
    top: 12px;
    right: 12px;
    color: var(--faint);
  }
  .cell.active .icon {
    color: var(--ok);
  }
  .cell.failed .icon {
    color: var(--danger);
  }
  .value {
    font-size: 32px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    padding-top: 6px;
  }
  :global(:root[data-theme="panel"]) .value {
    font-size: 28px;
  }
  :global(:root[data-theme="panel"]) .cell.active .value {
    color: var(--ok);
  }
  .value small {
    font-size: 0.45em;
    color: var(--muted);
    margin-left: 2px;
    font-weight: inherit;
    letter-spacing: 0;
  }
  .name {
    font-size: 12px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  :global(:root[data-theme="panel"]) .name,
  :global(:root[data-theme="swiss"]) .name {
    font-family: var(--font-mono);
    font-size: 10.5px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  .empty {
    color: var(--muted);
    font-size: 13px;
  }
</style>

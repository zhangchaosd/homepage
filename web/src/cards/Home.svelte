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
  const byDomain: Record<string, string> = {
    light: "bulb",
    switch: "power",
    fan: "wind",
    climate: "thermometer",
    lock: "key",
    person: "eye",
  };

  const words: Record<string, string> = {
    on: "开",
    off: "关",
    home: "在家",
    not_home: "外出",
    open: "开",
    closed: "关",
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
      <li class="tile" class:active={shown.active && !entity.error} class:failed={!!entity.error} title={entity.error ?? entity.id}>
        <span class="icon"><Icon name={iconFor(entity)} size={16} /></span>
        <span class="value tnum">
          {#if entity.error}—{:else}{shown.value}{#if shown.unit}<span class="unit">{shown.unit}</span>{/if}{/if}
        </span>
        <span class="name">{entity.name}</span>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .grid {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(124px, 1fr));
    gap: 8px;
  }
  .tile {
    display: grid;
    gap: 2px;
    padding: 12px 13px 11px;
    border-radius: var(--radius-tile);
    border: 1px solid var(--line);
    background: var(--surface-2);
    min-width: 0;
  }
  .icon {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    margin-bottom: 8px;
    border-radius: 9px;
    background: var(--surface);
    border: 1px solid var(--line);
    color: var(--muted);
  }
  .tile.active .icon {
    background: var(--accent-soft);
    border-color: transparent;
    color: var(--accent-ink);
  }
  .tile.failed .icon {
    color: var(--danger);
  }
  .value {
    font-size: 22px;
    font-weight: 600;
    letter-spacing: -0.03em;
    line-height: 1.15;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .unit {
    margin-left: 2px;
    font-size: 13px;
    font-weight: 500;
    color: var(--muted);
    letter-spacing: 0;
  }
  .name {
    font-size: 12px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .empty {
    color: var(--muted);
    font-size: 13px;
  }
</style>

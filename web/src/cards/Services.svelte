<script lang="ts">
  import Icon, { iconNames } from "../lib/Icon.svelte";
  import type { Health, Service, Slot } from "../lib/api";

  let { services, health, query = "" }: { services: Service[]; health?: Slot<Health>; query?: string } = $props();

  const aliases: Record<string, string> = {
    forgejo: "branch",
    gitea: "branch",
    git: "branch",
    vikunja: "todo",
    todo: "todo",
    homeassistant: "house",
    "home-assistant": "house",
    ha: "house",
    obsidian: "notebook",
    notes: "notebook",
    bookmarkd: "bookmark",
    bookmarks: "bookmark",
    github: "github",
    ipv6who: "network",
    inbox: "inbox",
    inboxd: "inbox",
    收件箱: "inbox",
    "cross-device-inbox": "inbox",
  };

  function glyph(service: Service): { kind: "file" | "icon" | "letter"; value: string } {
    const icon = service.icon?.trim();
    if (icon && /\.(svg|png|webp|jpe?g|ico|gif)$/i.test(icon)) return { kind: "file", value: `/icons/${encodeURIComponent(icon)}` };
    const key = (icon || service.name).toLowerCase().replace(/\s+/g, "");
    if (aliases[key]) return { kind: "icon", value: aliases[key] };
    if (icon && iconNames.includes(icon)) return { kind: "icon", value: icon };
    return { kind: "letter", value: Array.from(service.name.trim())[0]?.toUpperCase() ?? "?" };
  }

  function host(url: string): string {
    try {
      return new URL(url).host.replace(/^www\./, "");
    } catch {
      return url;
    }
  }

  const visible = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return services;
    return services.filter((service) =>
      [service.name, service.description ?? "", service.group ?? "", service.url].some((field) => field.toLowerCase().includes(q)),
    );
  });

  // One column grid for every group, so every tile has the same width: a lone
  // tile stays tile-sized instead of stretching across the row.
  let width = $state(0);
  const cols = $derived(width >= 1100 ? 6 : width >= 880 ? 5 : width >= 660 ? 4 : width >= 440 ? 3 : 2);

  const groups = $derived.by(() => {
    const map = new Map<string, Service[]>();
    for (const service of visible) {
      const group = service.group?.trim() || "";
      map.set(group, [...(map.get(group) ?? []), service]);
    }
    return [...map.entries()];
  });
</script>

{#if visible.length === 0}
  <p class="empty">没有匹配的服务</p>
{:else}
  <div class="groups" bind:clientWidth={width} style:--cols={cols}>
    {#each groups as [group, items] (group)}
      <!-- Groups share rows when they fit; a group spans as many columns as it has tiles. -->
      <section class="group" style:--span={Math.min(items.length, cols)}>
        {#if group && groups.length > 1}
          <h3 class="label">{group}</h3>
        {/if}
        <ul class="grid">
          {#each items as service (service.name)}
            {@const g = glyph(service)}
            {@const status = health?.data?.[service.name]}
            <li>
              <a class="tile pressable" href={service.url} target="_blank" rel="noopener noreferrer">
                <span class="glyph" class:letter={g.kind === "letter"}>
                  {#if g.kind === "file"}
                    <img src={g.value} alt="" width="20" height="20" loading="lazy" />
                  {:else if g.kind === "icon"}
                    <Icon name={g.value} size={18} />
                  {:else}
                    {g.value}
                  {/if}
                </span>
                <span class="text">
                  <span class="name">{service.name}</span>
                  <span class="sub">{service.description || host(service.url)}</span>
                </span>
                {#if status}
                  {#if status.up && status.latencyMs !== null}<span class="lat mono">{status.latencyMs} ms</span>{/if}
                  <span
                    class="dot"
                    class:down={!status.up}
                    title={status.up ? `在线 · ${status.latencyMs ?? "?"} ms` : `离线 · ${status.error ?? `HTTP ${status.status}`}`}
                  >
                    <span class="visually-hidden">{status.up ? "在线" : "离线"}</span>
                  </span>
                {/if}
              </a>
            </li>
          {/each}
        </ul>
      </section>
    {/each}
  </div>
{/if}

<style>
  /* Groups and the tiles inside them share one set of column tracks; the
     column gap equals the tile gap so every tile comes out the same width. */
  .groups {
    display: grid;
    grid-template-columns: repeat(var(--cols, 6), minmax(0, 1fr));
    gap: 18px var(--tile-gap);
  }
  .group {
    grid-column: span var(--span);
    display: grid;
    grid-template-rows: auto 1fr;
    align-content: start;
    min-width: 0;
  }
  h3 {
    margin: 0 0 10px;
  }

  /* A ruled table of tiles: 1px of the grid colour shows between cells. */
  /* Ruled cells: each tile draws its own hairline, so a short last row never
     shows the grid colour through empty cells. */
  .grid {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(var(--span), minmax(0, 1fr));
    gap: var(--tile-gap);
    align-content: start;
  }
  .grid li {
    outline: var(--tile-gap) solid var(--tile-grid-bg);
  }
  :global(:root[data-family="panel"]) .grid li {
    outline: 0;
  }

  .tile {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 68px;
    padding: 12px 14px;
    background: var(--tile-bg);
    border-radius: var(--tile-radius);
  }
  :global(:root[data-family="panel"]) .tile {
    border: 1px solid var(--tile-border);
  }
  @media (hover: hover) and (pointer: fine) {
    .tile:hover {
      background: var(--surface-hover);
    }
    :global(:root[data-theme="swiss"]) .tile:hover {
      background: var(--accent);
      color: var(--on-accent);
    }
    :global(:root[data-theme="swiss"]) .tile:hover .sub {
      color: var(--on-accent);
      opacity: 0.7;
    }
    :global(:root[data-theme="swiss"]) .tile:hover .glyph {
      background: var(--on-accent);
      color: var(--accent);
    }
  }

  /* Paper: the mark is bare ink. Panel: a keycap. Swiss: a black square. */
  .glyph {
    flex: none;
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    color: var(--ink);
    transition: background-color 160ms ease, color 160ms ease;
  }
  .glyph.letter {
    font-family: var(--font-number);
    font-weight: var(--w-number);
    font-size: 20px;
    font-style: italic;
  }
  .glyph img {
    object-fit: contain;
  }
  :global(:root[data-family="panel"]) .glyph {
    background: var(--bg);
    border: 1px solid var(--line);
    border-radius: 7px;
    color: var(--accent);
  }
  :global(:root[data-family="panel"]) .glyph.letter,
  :global(:root[data-theme="swiss"]) .glyph.letter {
    font-style: normal;
    font-size: 15px;
    font-weight: 700;
  }
  :global(:root[data-theme="swiss"]) .glyph {
    background: var(--ink);
    color: var(--bg);
  }

  .text {
    display: grid;
    min-width: 0;
    flex: 1;
  }
  .name {
    font-weight: 560;
    font-size: 13.5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .sub {
    font-size: 12px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transition: color 160ms ease;
  }
  .tile .dot {
    position: absolute;
    top: 12px;
    right: 12px;
  }
  /* The latency readout; themes that want it turn it on. */
  .lat {
    display: none;
    position: absolute;
    top: 10px;
    right: 26px;
    font-size: 10.5px;
    color: var(--muted);
  }

  .empty {
    margin: 8px 0;
    color: var(--muted);
    font-size: 13px;
  }

  @media (max-width: 760px) {
    .tile {
      gap: 9px;
      padding: 10px;
      min-height: 60px;
    }
    .glyph {
      width: 30px;
      height: 30px;
    }
  }
</style>

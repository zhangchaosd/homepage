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
  <div class="groups">
    {#each groups as [group, items] (group)}
      <!-- Groups share a row, each taking width in proportion to its size. -->
      <section class="group" style:--n={items.length}>
        {#if group && groups.length > 1}
          <h3>{group}</h3>
        {/if}
        <ul class="grid">
          {#each items as service (service.name)}
            {@const g = glyph(service)}
            {@const status = health?.data?.[service.name]}
            <li>
              <a class="tile pressable" href={service.url} target="_blank" rel="noopener noreferrer">
                <span class="glyph" class:letter={g.kind === "letter"}>
                  {#if g.kind === "file"}
                    <img src={g.value} alt="" width="22" height="22" loading="lazy" />
                  {:else if g.kind === "icon"}
                    <Icon name={g.value} size={19} />
                  {:else}
                    {g.value}
                  {/if}
                </span>
                <span class="text">
                  <span class="name">{service.name}</span>
                  <span class="sub">{service.description || host(service.url)}</span>
                </span>
                {#if status}
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
  .groups {
    display: flex;
    flex-wrap: wrap;
    gap: 14px 20px;
  }
  .group {
    flex: var(--n) 1 calc(var(--n) * 200px);
    min-width: 0;
  }
  h3 {
    margin: 2px 0 8px 2px;
    font-size: 11.5px;
    font-weight: 550;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--muted);
  }

  .grid {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(190px, 1fr));
    gap: 8px;
  }

  .tile {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px 10px 10px;
    border-radius: var(--radius-tile);
    border: 1px solid var(--line);
    background: var(--surface-2);
  }
  @media (hover: hover) and (pointer: fine) {
    .tile:hover {
      background: var(--surface-hover);
      border-color: var(--line-strong);
    }
    .tile:hover .glyph {
      color: var(--ink);
    }
  }

  .glyph {
    flex: none;
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    border-radius: 11px;
    background: var(--surface);
    border: 1px solid var(--line);
    color: var(--ink-2);
    box-shadow: 0 1px 1px rgba(0, 0, 0, 0.03);
    transition: color 160ms ease;
  }
  .glyph.letter {
    font-weight: 650;
    font-size: 15px;
  }
  .glyph img {
    border-radius: 5px;
    object-fit: contain;
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
  }

  .dot {
    position: absolute;
    top: 9px;
    right: 9px;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .dot.down {
    background: var(--danger);
    box-shadow: 0 0 0 3px var(--danger-soft);
  }

  .empty {
    margin: 8px 0;
    color: var(--muted);
    font-size: 13px;
  }
  @media (max-width: 760px) {
    .group {
      flex-basis: 100%;
    }
    /* Two compact columns on phones keep the launcher within a screen or so. */
    .grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
    .tile {
      gap: 9px;
      padding: 8px 9px 8px 8px;
    }
    .glyph {
      width: 32px;
      height: 32px;
      border-radius: 9px;
    }
  }
</style>

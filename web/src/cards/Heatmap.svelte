<script lang="ts">
  import { startOfDay } from "../lib/time";

  /** Daily counts keyed by local date `YYYY-MM-DD`. Cells keep a fixed size; as many
   *  weeks as fit the available width are shown, up to `maxWeeks`. */
  let {
    counts,
    maxWeeks = 26,
    end = new Date(),
    label,
    green = false,
    fill = false,
  }: { counts: Map<string, number>; maxWeeks?: number; end?: Date; label: string; green?: boolean; fill?: boolean } = $props();

  let available = $state(0);

  const CELL = 11;
  const GAP = 3;
  const fmt = new Intl.DateTimeFormat("zh-CN", { month: "short" });
  const dayFmt = new Intl.DateTimeFormat("zh-CN", { month: "long", day: "numeric" });

  function key(date: Date) {
    return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
  }

  const weeks = $derived(Math.max(8, Math.min(maxWeeks, Math.floor((available + GAP) / (CELL + GAP)) || maxWeeks)));

  const grid = $derived.by(() => {
    const last = startOfDay(end);
    // Columns start on Monday; the last column ends with today.
    const offset = (last.getDay() + 6) % 7;
    const first = new Date(last);
    first.setDate(first.getDate() - offset - (weeks - 1) * 7);
    const cells: { x: number; y: number; date: Date; count: number }[] = [];
    const months: { x: number; text: string }[] = [];
    let lastMonth = -1;
    for (let w = 0; w < weeks; w++) {
      for (let d = 0; d < 7; d++) {
        const date = new Date(first);
        date.setDate(first.getDate() + w * 7 + d);
        if (date > last) break;
        if (d === 0 && date.getMonth() !== lastMonth) {
          if (w < weeks - 2) months.push({ x: w * (CELL + GAP), text: fmt.format(date) });
          lastMonth = date.getMonth();
        }
        cells.push({ x: w * (CELL + GAP), y: d * (CELL + GAP), date, count: counts.get(key(date)) ?? 0 });
      }
    }
    // Levels follow the distribution of non-zero days so one busy day does not flatten the rest.
    const values = cells.map((c) => c.count).filter((v) => v > 0).sort((a, b) => a - b);
    const q = (p: number) => values[Math.min(values.length - 1, Math.floor(p * values.length))] ?? 1;
    const thresholds = [q(0.25), q(0.5), q(0.8)];
    const level = (v: number) => (v <= 0 ? 0 : v <= thresholds[0] ? 1 : v <= thresholds[1] ? 2 : v <= thresholds[2] ? 3 : 4);
    return { cells: cells.map((c) => ({ ...c, level: level(c.count) })), months };
  });

  const width = $derived(weeks * (CELL + GAP) - GAP);
  const height = 7 * (CELL + GAP) - GAP + 16;
</script>

<div class="fit" bind:clientWidth={available}>
<!-- `fill` scales the whole calendar to the available width (cells up to about 1.6x). -->
<svg
  viewBox="0 0 {width} {height}"
  width={fill ? "100%" : width}
  height={fill ? undefined : height}
  style:max-width={fill ? `${Math.round(width * 1.6)}px` : null}
  class="heatmap"
  class:green role="img" aria-label={label}>
  {#each grid.months as month (month.x)}
    <text x={month.x} y="9" class="month">{month.text}</text>
  {/each}
  <g transform="translate(0 16)">
    {#each grid.cells as cell (cell.x * 10 + cell.y)}
      <rect x={cell.x} y={cell.y} width={CELL} height={CELL} class="l{cell.level}">
        <title>{dayFmt.format(cell.date)} · {cell.count} 次贡献</title>
      </rect>
    {/each}
  </g>
</svg>
</div>

<style>
  .fit {
    width: 100%;
    min-width: 0;
  }
  .heatmap {
    display: block;
    overflow: visible;
  }
  .month {
    font-size: 9.5px;
    fill: var(--muted);
    font-family: var(--font-mono);
    letter-spacing: 0.06em;
  }
  rect {
    fill: var(--heat-0);
    rx: var(--heat-radius, 1.5px);
  }
  :global(:root[data-family="panel"]) rect {
    rx: 1px;
  }
  :global(:root[data-theme="swiss"]) rect {
    rx: 0;
  }
  /* GitHub's own greens, whatever the theme: the calendar reads as GitHub's. */
  .green {
    --heat-1: light-dark(#9be9a8, #0e4429);
    --heat-2: light-dark(#40c463, #006d32);
    --heat-3: light-dark(#30a14e, #26a641);
    --heat-4: light-dark(#216e39, #39d353);
  }
  .l1 {
    fill: var(--heat-1);
  }
  .l2 {
    fill: var(--heat-2);
  }
  .l3 {
    fill: var(--heat-3);
  }
  .l4 {
    fill: var(--heat-4);
  }
</style>

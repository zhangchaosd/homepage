<script lang="ts">
  import Icon from "../lib/Icon.svelte";
  import type { Deepseek } from "../lib/api";

  let { data }: { data: Deepseek } = $props();

  const symbol: Record<string, string> = { CNY: "¥", USD: "$" };
  const main = $derived(data.balances[0] ?? null);
  const others = $derived(data.balances.slice(1));

  // "110.00" → ["110", "00"], the fraction set smaller.
  function split(value: string): [string, string] {
    const [whole, fraction = ""] = value.split(".");
    return [whole, fraction];
  }
</script>

{#if !main}
  <p class="empty">没有余额信息</p>
{:else}
  {@const [whole, fraction] = split(main.totalBalance)}
  <p class="summary">
    <span class="figure n tnum"><small class="cur">{symbol[main.currency] ?? main.currency}</small>{whole}{#if fraction}<small class="frac">.{fraction}</small>{/if}</span>
    <span class="caption">余额 · {main.currency}</span>
  </p>
  <dl class="facts">
    <div>
      <dt class="label">状态</dt>
      <dd class:ok={data.available} class:down={!data.available}><i class="dot" class:down={!data.available}></i>{data.available ? "可用" : "不可用"}</dd>
    </div>
    <div>
      <dt class="label">充值余额</dt>
      <dd class="tnum">{symbol[main.currency] ?? ""}{main.toppedUpBalance}</dd>
    </div>
    <div>
      <dt class="label">赠送余额</dt>
      <dd class="tnum">{symbol[main.currency] ?? ""}{main.grantedBalance}</dd>
    </div>
    {#each others as other (other.currency)}
      <div>
        <dt class="label">{other.currency}</dt>
        <dd class="tnum">{symbol[other.currency] ?? ""}{other.totalBalance}</dd>
      </div>
    {/each}
  </dl>
  {#if !data.available}
    <p class="warn"><Icon name="alert" size={12} />余额不足，API 调用会被拒绝</p>
  {/if}
{/if}

<style>
  .summary {
    display: flex;
    align-items: baseline;
    gap: 12px;
    margin: 0 0 16px;
  }
  .n {
    font-size: 64px;
  }
  :global(:root[data-family="panel"]) .n {
    font-size: 54px;
    color: var(--accent);
  }
  .cur {
    font-size: 0.45em;
    margin-right: 0.1em;
    color: var(--muted);
  }
  .frac {
    font-size: 0.45em;
    color: var(--muted);
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

  /* Facts in a ruled strip, like the GitHub stats. */
  .facts {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    margin: 0;
    border-top: 1px solid var(--hair);
  }
  :global(:root[data-family="panel"]) .facts,
  :global(:root[data-theme="swiss"]) .facts {
    gap: var(--tile-gap);
    border: 0;
  }
  .facts div {
    display: grid;
    gap: 6px;
    padding: 12px 12px 10px 0;
    border-right: 1px solid var(--hair);
  }
  .facts div:last-child {
    border-right: 0;
  }
  .facts div + div {
    padding-left: 12px;
  }
  :global(:root[data-family="panel"]) .facts div,
  :global(:root[data-theme="swiss"]) .facts div {
    padding: 10px 12px;
    border: 1px solid var(--tile-border);
    border-radius: var(--tile-radius);
    background: var(--tile-bg);
  }
  dd {
    margin: 0;
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 16px;
    font-weight: 560;
  }
  dd.ok {
    color: var(--ok);
  }
  dd.down {
    color: var(--danger);
  }
  .warn {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 12px 0 0;
    font-size: 12.5px;
    color: var(--warn);
  }
  .empty {
    color: var(--muted);
    font-size: 13px;
  }
</style>

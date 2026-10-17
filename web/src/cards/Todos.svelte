<script lang="ts">
  import Icon from "../lib/Icon.svelte";
  import TodoDialog from "./TodoDialog.svelte";
  import { completeTodo, type Task, type Todos } from "../lib/api";
  import { dayDiff, dueLabel } from "../lib/time";
  import { now } from "../lib/clock.svelte";

  let { data, onchange }: { data: Todos; onchange: () => void } = $props();

  // Ids being completed (optimistic) and ids that finished, hidden until the next refresh.
  let pending = $state<Record<number, boolean>>({});
  let done = $state<Record<number, boolean>>({});
  let failed = $state<string>("");
  let adding = $state(false);

  const groups = $derived.by(() => {
    const today = new Date(now.value);
    const overdue: Task[] = [];
    const due: Task[] = [];
    const upcoming: Task[] = [];
    for (const task of data.tasks) {
      if (done[task.id] || !task.due) continue;
      const diff = dayDiff(new Date(task.due), today);
      const past = new Date(task.due).getTime() < today.getTime();
      if (diff < 0 || (diff === 0 && past && hasTime(task.due))) overdue.push(task);
      else if (diff === 0) due.push(task);
      else upcoming.push(task);
    }
    return [
      { key: "overdue", label: "已逾期", tasks: overdue },
      { key: "today", label: "今天", tasks: due },
      { key: "upcoming", label: `接下来 ${data.upcomingDays} 天`, tasks: upcoming },
    ].filter((group) => group.tasks.length > 0);
  });

  const todayCount = $derived(groups.filter((g) => g.key !== "upcoming").reduce((sum, g) => sum + g.tasks.length, 0));

  function hasTime(iso: string) {
    const date = new Date(iso);
    return !(date.getHours() === 0 && date.getMinutes() === 0);
  }

  async function complete(task: Task) {
    if (pending[task.id]) return;
    pending[task.id] = true;
    failed = "";
    try {
      await completeTodo(task.id);
      // Let the check animation land before the row leaves.
      setTimeout(() => {
        done[task.id] = true;
        delete pending[task.id];
        onchange();
      }, 450);
    } catch (cause) {
      delete pending[task.id];
      failed = cause instanceof Error ? cause.message : "无法完成任务";
    }
  }
</script>

{#if adding}
  <!-- The server refreshes Vikunja right after a create; pick that up shortly. -->
  <TodoDialog onclose={() => (adding = false)} oncreated={() => setTimeout(onchange, 700)} />
{/if}

<div class="summary">
  <span class="figure n tnum">{todayCount}</span>
  <span class="caption">{todayCount === 0 ? "今天没有待办，享受一下" : "项待办需要今天处理"}</span>
  <button class="add btn pressable" onclick={() => (adding = true)} aria-label="添加待办">
    <Icon name="plus" size={13} stroke={2.2} /><span>添加</span>
  </button>
</div>

{#if groups.length === 0}
  <div class="clear"><Icon name="check" size={16} stroke={2.2} /><span>全部完成</span></div>
{:else}
  <div class="groups">
    {#each groups as group (group.key)}
      <div class="group">
        <h3 class="label" class:overdue={group.key === "overdue"}>{group.label}<span class="count tnum">{group.tasks.length}</span></h3>
        <ul>
          {#each group.tasks as task (task.id)}
            <li class:checked={pending[task.id]}>
              <button
                class="check pressable"
                class:urgent={task.priority >= 4 || group.key === "overdue"}
                onclick={() => complete(task)}
                aria-label="完成 {task.title}"
                disabled={pending[task.id]}
              >
                <Icon name="check" size={10} stroke={3} />
              </button>
              <a class="task" href={task.url} target="_blank" rel="noopener noreferrer">
                <span class="title">{task.title}</span>
                <span class="meta">
                  {#if task.due}<span class="due tnum" class:late={group.key === "overdue"}>{dueLabel(task.due, new Date(now.value))}</span>{/if}
                  {#if task.project}<span>{task.project}</span>{/if}
                  {#each task.labels.slice(0, 2) as label (label.title)}
                    <span class="tag" style:--label={label.hex_color ? `#${label.hex_color.replace("#", "")}` : null}>{label.title}</span>
                  {/each}
                </span>
              </a>
            </li>
          {/each}
        </ul>
      </div>
    {/each}
  </div>
{/if}

{#if failed}<p class="error" role="alert">{failed}</p>{/if}

<style>
  .summary {
    display: flex;
    align-items: baseline;
    gap: 12px;
    margin: 0 0 16px;
  }
  .add {
    align-self: center;
    margin-left: auto;
    height: 30px;
    padding: 0 12px;
  }
  .n {
    font-size: 64px;
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

  .groups {
    display: grid;
    gap: 14px;
  }
  h3 {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin: 0 0 2px;
  }
  h3.overdue {
    color: var(--danger);
  }
  .count {
    opacity: 0.7;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 9px 0;
    border-top: 1px solid var(--hair-soft);
    transition: opacity 300ms ease;
  }
  li:first-child {
    border-top: 0;
  }
  li.checked {
    opacity: 0.4;
  }

  /* Paper: a ring. Panel/swiss: a box. */
  .check {
    flex: none;
    display: grid;
    place-items: center;
    width: 16px;
    height: 16px;
    margin-top: 3px;
    border: 1px solid var(--faint);
    border-radius: 50%;
    color: transparent;
    transition:
      transform 160ms var(--ease-out),
      background-color 180ms ease,
      border-color 180ms ease,
      color 180ms ease;
  }
  :global(:root[data-family="panel"]) .check {
    border-radius: 3px;
  }
  :global(:root[data-theme="swiss"]) .check {
    border-radius: 0;
    border: 1.5px solid var(--ink);
  }
  .check.urgent {
    border-color: var(--danger);
  }
  @media (hover: hover) and (pointer: fine) {
    .check:hover {
      border-color: var(--ink);
      color: var(--ink);
    }
  }
  li.checked .check {
    background: var(--ink);
    border-color: var(--ink);
    color: var(--bg);
  }

  .task {
    display: grid;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }
  .title {
    font-size: 14px;
    line-height: 1.4;
    overflow-wrap: anywhere;
    transition: color 160ms ease;
  }
  li.checked .title {
    text-decoration: line-through;
    text-decoration-color: var(--faint);
  }
  @media (hover: hover) and (pointer: fine) {
    .task:hover .title {
      color: var(--accent-ink);
    }
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 2px 10px;
    font-size: 12px;
    color: var(--muted);
  }
  :global(:root[data-family="panel"]) .meta {
    font-family: var(--font-mono);
    font-size: 11.5px;
  }
  .due.late {
    color: var(--danger);
  }
  .tag {
    --label: var(--muted);
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  .tag::before {
    content: "";
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--label);
  }

  .clear {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 20px 0;
    color: var(--ok);
    font-weight: 550;
  }
  .error {
    margin: 10px 0 0;
    color: var(--danger);
    font-size: 12.5px;
  }
</style>

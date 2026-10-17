<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "../lib/Icon.svelte";
  import Sheet from "../lib/Sheet.svelte";
  import { createTodo, getTodoOptions, type TodoOptions } from "../lib/api";

  /** Adds one Vikunja task: title, due date (quick picks or a date and an
   *  optional time), project, priority, existing labels and new ones. */
  let { onclose, oncreated }: { onclose: () => void; oncreated: () => void } = $props();

  let sheet: Sheet | undefined = $state();
  let options = $state<TodoOptions | null>(null);
  let loadError = $state("");

  let title = $state("");
  let date = $state("");
  let time = $state("");
  let project = $state<number | null>(null);
  let priority = $state(0);
  let picked = $state<number[]>([]);
  let fresh = $state<string[]>([]);
  let labelText = $state("");

  let saving = $state(false);
  let error = $state("");
  let titleInput: HTMLInputElement | undefined = $state();

  const priorities = [
    { value: 0, name: "无" },
    { value: 1, name: "低" },
    { value: 2, name: "中" },
    { value: 3, name: "高" },
    { value: 4, name: "紧急" },
  ];

  function iso(day: Date) {
    return `${day.getFullYear()}-${String(day.getMonth() + 1).padStart(2, "0")}-${String(day.getDate()).padStart(2, "0")}`;
  }
  function plus(days: number) {
    const day = new Date();
    day.setDate(day.getDate() + days);
    return iso(day);
  }
  // Quick picks; the weekend is the coming Saturday, next week its Monday.
  const weekday = new Date().getDay();
  const quick = [
    { name: "今天", value: plus(0) },
    { name: "明天", value: plus(1) },
    { name: "周末", value: plus((6 - weekday + 7) % 7) },
    { name: "下周一", value: plus(((8 - weekday) % 7) || 7) },
  ];

  const canSave = $derived(!saving && title.trim().length > 0 && options !== null);
  const query = $derived(labelText.trim().toLowerCase());
  const exact = $derived(options?.labels.find((label) => label.title.toLowerCase() === query));

  onMount(async () => {
    titleInput?.focus();
    try {
      options = await getTodoOptions();
      project = options.defaultProject;
    } catch (cause) {
      loadError = cause instanceof Error ? cause.message : "无法读取项目和标签";
    }
  });

  function toggle(id: number) {
    picked = picked.includes(id) ? picked.filter((value) => value !== id) : [...picked, id];
  }

  function addLabel() {
    const name = labelText.trim();
    if (!name) return;
    if (exact) {
      if (!picked.includes(exact.id)) picked = [...picked, exact.id];
    } else if (!fresh.some((value) => value.toLowerCase() === name.toLowerCase())) {
      fresh = [...fresh, name];
    }
    labelText = "";
  }

  function onLabelKey(event: KeyboardEvent) {
    if (event.key === "Enter" && !event.metaKey && !event.ctrlKey) {
      event.preventDefault();
      addLabel();
    } else if (event.key === "Backspace" && !labelText && fresh.length > 0) {
      fresh = fresh.slice(0, -1);
    }
  }

  async function save(event?: Event) {
    event?.preventDefault();
    if (!canSave) return;
    if (labelText.trim()) addLabel();
    saving = true;
    error = "";
    try {
      // A date alone means the whole day (midnight, local time).
      const due = date ? new Date(`${date}T${time || "00:00"}`).toISOString() : null;
      await createTodo({ title: title.trim(), projectId: project, due, priority, labels: picked, newLabels: fresh });
      saving = false;
      oncreated();
      sheet?.close();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "添加失败";
      saving = false;
    }
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) void save(event);
  }
</script>

<Sheet bind:this={sheet} label="添加待办" busy={saving} width={520} {onclose}>
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <form id="todo-form" onsubmit={save} onkeydown={onKeydown}>
    <input
      bind:this={titleInput}
      bind:value={title}
      class="title-input"
      placeholder="要做什么？"
      maxlength="250"
      autocomplete="off"
      aria-label="标题"
    />

    <fieldset>
      <legend class="label">截止</legend>
      <div class="chips">
        {#each quick as pick (pick.name)}
          <button type="button" class="chip pressable" class:on={date === pick.value} onclick={() => (date = date === pick.value ? "" : pick.value)}>
            {pick.name}
          </button>
        {/each}
        <button type="button" class="chip pressable" class:on={!date} onclick={() => ((date = ""), (time = ""))}>不设</button>
      </div>
      <div class="when">
        <input type="date" bind:value={date} aria-label="日期" />
        <input type="time" bind:value={time} disabled={!date} aria-label="时间（可选）" />
      </div>
    </fieldset>

    <fieldset>
      <legend class="label">优先级</legend>
      <div class="segmented" role="radiogroup" aria-label="优先级">
        {#each priorities as item (item.value)}
          <button
            type="button"
            role="radio"
            aria-checked={priority === item.value}
            class:on={priority === item.value}
            class:urgent={item.value === 4}
            onclick={() => (priority = item.value)}>{item.name}</button
          >
        {/each}
      </div>
    </fieldset>

    {#if options && options.projects.length > 1}
      <fieldset>
        <legend class="label">项目</legend>
        <select bind:value={project} aria-label="项目">
          {#each options.projects as item (item.id)}
            <option value={item.id}>{item.title}</option>
          {/each}
        </select>
      </fieldset>
    {/if}

    <fieldset>
      <legend class="label">标签</legend>
      {#if options}
        <div class="chips">
          {#each options.labels as label (label.id)}
            <button
              type="button"
              class="chip tag pressable"
              class:on={picked.includes(label.id)}
              class:match={query && label.title.toLowerCase().includes(query)}
              style:--label={label.hex_color ? `#${label.hex_color.replace("#", "")}` : null}
              aria-pressed={picked.includes(label.id)}
              onclick={() => toggle(label.id)}>{label.title}</button
            >
          {/each}
          {#each fresh as name (name)}
            <button type="button" class="chip tag on fresh pressable" onclick={() => (fresh = fresh.filter((value) => value !== name))} aria-label="移除新标签 {name}">
              {name}<span class="new">新</span><Icon name="x" size={10} stroke={2.4} />
            </button>
          {/each}
        </div>
      {/if}
      <div class="new-label">
        <Icon name="tag" size={13} />
        <input bind:value={labelText} onkeydown={onLabelKey} placeholder="输入标签，回车添加；没有的会新建" maxlength="250" autocomplete="off" aria-label="新标签" />
        {#if labelText.trim()}
          <button type="button" class="add-label" onclick={addLabel}>{exact ? "选上" : "新建"}</button>
        {/if}
      </div>
    </fieldset>

    {#if loadError}<p class="error" role="alert">{loadError}</p>{/if}
  </form>

  {#snippet footer()}
    <p class="message" role="status" aria-live="polite">
      {#if error}<span class="error">{error}</span>{:else}<span class="hint label"><kbd>⌘</kbd><kbd>↵</kbd> 添加</span>{/if}
    </p>
    <button class="btn" type="button" onclick={() => sheet?.close()} disabled={saving}>取消</button>
    <button class="btn primary" type="submit" form="todo-form" disabled={!canSave}>
      <Icon name="plus" size={13} stroke={2.2} />{saving ? "添加中…" : "添加"}
    </button>
  {/snippet}
</Sheet>

<style>
  form {
    display: grid;
    gap: 18px;
  }
  .title-input {
    width: 100%;
    height: 44px;
    padding: 0 2px;
    border: 0;
    border-bottom: 1px solid var(--line);
    background: transparent;
    outline: none;
    font-size: 18px;
    font-weight: 520;
    transition: border-color 160ms ease;
  }
  .title-input:focus {
    border-color: var(--accent);
  }
  .title-input::placeholder {
    color: var(--faint);
  }

  fieldset {
    display: grid;
    gap: 9px;
    margin: 0;
    padding: 0;
    border: 0;
    min-width: 0;
  }
  legend {
    padding: 0;
    margin-bottom: 9px;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    --label: var(--muted);
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 11px;
    border: 1px solid var(--hair);
    border-radius: var(--chip-radius);
    font-size: 12.5px;
    color: var(--ink-2);
    background: transparent;
  }
  .chip.on {
    border-color: var(--ink);
    background: var(--ink);
    color: var(--bg);
  }
  .chip.tag::before {
    content: "";
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--label);
  }
  .chip.match:not(.on) {
    border-color: var(--accent);
  }
  .chip .new {
    font-size: 10px;
    opacity: 0.7;
  }
  @media (hover: hover) and (pointer: fine) {
    .chip:not(.on):hover {
      border-color: var(--line);
      color: var(--ink);
    }
  }

  .when {
    display: flex;
    gap: 8px;
  }
  .when input,
  select {
    height: 34px;
    padding: 0 10px;
    border: 1px solid var(--input-border);
    border-radius: var(--input-radius);
    background: var(--input-bg);
    font-size: 13.5px;
    color: var(--ink);
    color-scheme: inherit;
  }
  .when input[type="date"] {
    flex: 1;
    min-width: 0;
  }
  .when input:disabled {
    opacity: 0.45;
  }
  select {
    width: 100%;
  }

  .segmented {
    display: inline-flex;
    width: fit-content;
    border: 1px solid var(--hair);
    border-radius: var(--btn-radius);
    overflow: hidden;
  }
  .segmented button {
    height: 30px;
    padding: 0 14px;
    font-size: 12.5px;
    color: var(--ink-2);
    border-left: 1px solid var(--hair);
    transition: background-color 140ms ease, color 140ms ease;
  }
  .segmented button:first-child {
    border-left: 0;
  }
  .segmented button.on {
    background: var(--ink);
    color: var(--bg);
  }
  .segmented button.on.urgent {
    background: var(--danger);
    color: #fff;
  }

  .new-label {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 34px;
    padding: 0 6px 0 10px;
    border: 1px dashed var(--hair);
    border-radius: var(--input-radius);
    color: var(--muted);
  }
  .new-label:focus-within {
    border-style: solid;
    border-color: var(--accent);
  }
  .new-label input {
    flex: 1;
    min-width: 0;
    height: 100%;
    border: 0;
    background: transparent;
    outline: none;
    font-size: 16px; /* prevents iOS zoom on focus */
    color: var(--ink);
  }
  @media (hover: hover) and (pointer: fine) {
    .new-label input {
      font-size: 13px;
    }
  }
  .add-label {
    height: 24px;
    padding: 0 9px;
    border-radius: var(--chip-radius);
    background: var(--accent);
    color: var(--on-accent);
    font-size: 12px;
    font-weight: 550;
  }

  .message {
    flex: 1;
    min-width: 0;
    margin: 0;
    font-size: 12.5px;
  }
  .hint {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  @media (hover: none) {
    .hint {
      display: none;
    }
  }
  kbd {
    display: inline-grid;
    place-items: center;
    min-width: 16px;
    height: 16px;
    padding: 0 3px;
    border: 1px solid var(--hair);
    border-radius: var(--chip-radius);
    font: 10px var(--font-mono);
  }
  .error {
    margin: 0;
    color: var(--danger);
    font-size: 12.5px;
  }
</style>

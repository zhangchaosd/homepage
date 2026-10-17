<script lang="ts">
  import { onMount } from "svelte";
  import Login from "./Login.svelte";
  import Dashboard from "./Dashboard.svelte";
  import { ApiError, getDashboard, getSystem, restoreSession, type Dashboard as DashboardData, type SystemInfo } from "./lib/api";

  type View = { kind: "loading" } | { kind: "login"; system: SystemInfo } | { kind: "dashboard"; data: DashboardData } | { kind: "error"; message: string };

  let view = $state<View>({ kind: "loading" });

  async function enter() {
    await restoreSession();
    const data = await getDashboard();
    if (data.profile.title) document.title = data.profile.title;
    view = { kind: "dashboard", data };
  }

  async function boot() {
    try {
      await enter();
    } catch (cause) {
      if (cause instanceof ApiError && cause.status === 401) {
        try {
          const system = await getSystem();
          document.title = system.title;
          view = { kind: "login", system };
        } catch (inner) {
          view = { kind: "error", message: inner instanceof Error ? inner.message : "无法连接服务器" };
        }
      } else {
        view = { kind: "error", message: cause instanceof Error ? cause.message : "无法连接服务器" };
      }
    }
  }

  onMount(boot);
</script>

{#if view.kind === "login"}
  <Login system={view.system} onsuccess={enter} />
{:else if view.kind === "dashboard"}
  <Dashboard initial={view.data} onsignedout={boot} />
{:else if view.kind === "error"}
  <main class="center">
    <p>{view.message}</p>
    <button class="pressable" onclick={boot}>重试</button>
  </main>
{/if}

<style>
  .center {
    min-height: 100dvh;
    display: grid;
    place-items: center;
    align-content: center;
    gap: 12px;
    color: var(--muted);
  }
  .center p {
    margin: 0;
  }
  .center button {
    padding: 8px 16px;
    border-radius: 10px;
    border: 1px solid var(--line-strong);
    background: var(--surface);
    color: var(--ink);
  }
</style>

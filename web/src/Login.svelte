<script lang="ts">
  import Icon from "./lib/Icon.svelte";
  import { ApiError, PasskeyCancelled, login, passkeyLogin, type SystemInfo } from "./lib/api";
  import { greeting } from "./lib/time";

  let { system, onsuccess }: { system: SystemInfo; onsuccess: () => Promise<void> } = $props();

  let username = $state("");
  let password = $state("");
  let remember = $state(true);
  let busy = $state<"" | "password" | "passkey">("");
  let message = $state("");
  let shake = $state(0);

  const methods = $derived(system.auth);
  const canSubmit = $derived(!busy && password.length > 0 && (!methods.username || username.trim().length > 0));
  const date = new Date();
  const dateLine = new Intl.DateTimeFormat("zh-CN", { month: "long", day: "numeric", weekday: "long" }).format(date);

  function explain(cause: unknown, passkey: boolean): string {
    if (cause instanceof ApiError) {
      if (cause.status === 429) return "尝试次数过多，请稍后再试。";
      if (cause.status === 401) return passkey ? "这个 Passkey 未被接受。请使用在 bookmarkd 中为该域名注册的 Passkey。" : "用户名或密码不正确。";
      return cause.message;
    }
    return cause instanceof Error ? cause.message : "登录失败，请重试。";
  }

  async function withPassword(event: SubmitEvent) {
    event.preventDefault();
    if (!canSubmit) return;
    busy = "password";
    message = "";
    try {
      await login(methods.username ? username.trim() : null, password, remember);
      await onsuccess();
    } catch (cause) {
      message = explain(cause, false);
      password = "";
      shake++;
    } finally {
      busy = "";
    }
  }

  async function withPasskey() {
    if (busy) return;
    busy = "passkey";
    message = "";
    try {
      await passkeyLogin(remember);
      await onsuccess();
    } catch (cause) {
      message = cause instanceof PasskeyCancelled ? cause.message : explain(cause, true);
      if (!(cause instanceof PasskeyCancelled)) shake++;
    } finally {
      busy = "";
    }
  }
</script>

<main class="stage">
  <div class="sheet">
    <header class="mast">
      <span class="brand display">Home<span class="period">.</span></span>
      <span class="label">{dateLine}</span>
    </header>
    <div class="body">
      <div class="hello">
        <h1 class="display">{greeting(date)}<em>.</em></h1>
        <p class="lede">{system.title} · 登录以继续</p>
      </div>
      {#key shake}
        <div class="form" class:shake={shake > 0}>
          {#if methods.passkey}
            <button class="primary pressable" type="button" onclick={withPasskey} disabled={!!busy}>
              <Icon name="key" size={15} />
              {busy === "passkey" ? "等待 Passkey…" : "使用 Passkey 登录"}
            </button>
          {/if}
          {#if methods.passkey && methods.password}
            <div class="divider label"><span>或使用密码</span></div>
          {/if}
          {#if methods.password}
            <form onsubmit={withPassword}>
              {#if methods.username}
                <label>
                  <span class="label">用户名</span>
                  <input autocomplete="username" autocapitalize="none" spellcheck="false" bind:value={username} disabled={!!busy} />
                </label>
              {/if}
              <label>
                <span class="label">密码</span>
                <input type="password" autocomplete="current-password" bind:value={password} disabled={!!busy} />
              </label>
              <button class="{methods.passkey ? 'secondary' : 'primary'} pressable" type="submit" disabled={!canSubmit}>
                {busy === "password" ? "正在登录…" : "登录"}
                <Icon name="arrow-up-right" size={14} />
              </button>
            </form>
          {/if}
          <label class="remember">
            <input type="checkbox" bind:checked={remember} />
            <span class="box" aria-hidden="true"><Icon name="check" size={10} stroke={3} /></span>
            30 天内保持登录
          </label>
          <p class="message" role="alert" aria-live="polite">{message}</p>
        </div>
      {/key}
    </div>
  </div>
</main>

<style>
  .stage {
    position: relative;
    z-index: 1;
    min-height: 100dvh;
    display: grid;
    place-items: center;
    padding: 24px var(--gutter) calc(24px + env(safe-area-inset-bottom));
  }
  .sheet {
    width: min(100%, 880px);
    animation: rise 480ms var(--ease-out) both;
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
  }
  :global(:root[data-family="panel"]) .sheet,
  :global(:root[data-theme="swiss"]) .sheet {
    width: min(100%, 760px);
    background: var(--card-bg);
    border: var(--card-border-w) solid var(--card-border);
    border-radius: var(--card-radius);
    box-shadow: var(--card-shadow);
    padding: 0 28px 28px;
  }

  .mast {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    padding: 12px 0;
    border-bottom: 1px solid var(--line);
  }
  :global(:root[data-family="panel"]) .mast {
    border-bottom-color: var(--hair);
  }
  .brand {
    font-size: 20px;
    font-style: italic;
  }
  .brand .period {
    color: var(--accent);
  }
  :global(:root[data-family="panel"]) .brand,
  :global(:root[data-theme="swiss"]) .brand {
    font-style: normal;
    font-family: var(--font-mono);
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.14em;
    text-transform: uppercase;
  }

  .body {
    display: grid;
    grid-template-columns: 1.1fr 1fr;
    gap: 56px;
    align-items: center;
    padding: clamp(32px, 6vw, 72px) 0 24px;
  }
  @media (max-width: 760px) {
    .body {
      grid-template-columns: 1fr;
      gap: 28px;
      padding-top: 32px;
    }
  }
  h1 {
    margin: 0;
    font-size: clamp(52px, 7vw, 92px);
    line-height: 0.95;
  }
  h1 em {
    font-style: normal;
    color: var(--accent);
  }
  .lede {
    margin: 18px 0 0;
    color: var(--ink-2);
    font-size: 15px;
  }

  .form {
    display: grid;
    gap: 14px;
    max-width: 360px;
  }
  .form.shake {
    animation: shake 360ms var(--ease-out);
  }
  @keyframes shake {
    20% {
      transform: translateX(-6px);
    }
    40% {
      transform: translateX(5px);
    }
    60% {
      transform: translateX(-3px);
    }
    80% {
      transform: translateX(2px);
    }
  }

  .primary,
  .secondary {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: 100%;
    height: 44px;
    font-weight: 550;
    font-size: 14px;
    border-radius: var(--btn-radius);
  }
  .primary {
    background: var(--btn-bg);
    color: var(--btn-fg);
  }
  .secondary {
    border: var(--card-border-w, 1px) solid var(--line);
    background: transparent;
  }
  :global(:root[data-theme="paper"]) .secondary,
  :global(:root:not([data-theme])) .secondary {
    border-width: 1px;
  }
  /* Not ready yet: a quiet outline rather than a dimmed black block. */
  .primary:disabled,
  .secondary:disabled {
    background: transparent;
    color: var(--muted);
    border: 1px solid var(--hair);
    cursor: default;
    transform: none;
  }

  .divider {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 4px 0;
  }
  .divider::before,
  .divider::after {
    content: "";
    flex: 1;
    height: 1px;
    background: var(--hair);
  }

  form {
    display: grid;
    gap: 14px;
  }
  form label {
    display: grid;
    gap: 7px;
  }
  input:not([type="checkbox"]) {
    height: 42px;
    padding: 0 12px;
    border: var(--card-border-w, 1px) solid var(--input-border);
    border-radius: var(--input-radius);
    background: var(--input-bg);
    font-size: 16px; /* prevents iOS zoom on focus */
    outline: none;
    transition: border-color 160ms ease;
  }
  :global(:root[data-theme="paper"]) input:not([type="checkbox"]),
  :global(:root:not([data-theme])) input:not([type="checkbox"]) {
    border-width: 0 0 1px;
    padding: 0 2px;
    background: transparent;
  }
  input:not([type="checkbox"]):focus {
    border-color: var(--accent);
  }
  form button {
    margin-top: 6px;
  }

  .remember {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
    color: var(--ink-2);
    cursor: pointer;
    -webkit-user-select: none;
    user-select: none;
  }
  .remember input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }
  .box {
    display: grid;
    place-items: center;
    width: 16px;
    height: 16px;
    border: 1px solid var(--line);
    border-radius: var(--chip-radius);
    color: transparent;
    transition:
      background-color 140ms ease,
      color 140ms ease;
  }
  :global(:root[data-theme="paper"]) .box,
  :global(:root:not([data-theme])) .box {
    border-radius: 50%;
  }
  .remember input:checked + .box {
    background: var(--ink);
    color: var(--bg);
  }
  .remember input:focus-visible + .box {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .message {
    min-height: 20px;
    margin: 0;
    font-size: 13px;
    color: var(--danger);
  }
</style>

<script lang="ts">
  import Icon from "./lib/Icon.svelte";
  import { ApiError, PasskeyCancelled, login, passkeyLogin, type SystemInfo } from "./lib/api";

  let { system, onsuccess }: { system: SystemInfo; onsuccess: () => Promise<void> } = $props();

  let username = $state("");
  let password = $state("");
  let remember = $state(true);
  let busy = $state<"" | "password" | "passkey">("");
  let message = $state("");
  let shake = $state(0);

  const methods = $derived(system.auth);
  const canSubmit = $derived(!busy && password.length > 0 && (!methods.username || username.trim().length > 0));

  function explain(cause: unknown, passkey: boolean): string {
    if (cause instanceof ApiError) {
      if (cause.status === 429) return "尝试次数过多，请稍后再试。";
      if (cause.status === 401)
        return passkey ? "这个 Passkey 未被接受。请使用在 bookmarkd 中为该域名注册的 Passkey。" : "用户名或密码不正确。";
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
  {#key shake}
    <div class="panel" class:shake={shake > 0}>
      <div class="mark" aria-hidden="true">
        <svg viewBox="0 0 64 64" width="44" height="44"
          ><rect width="64" height="64" rx="16" fill="currentColor" /><path
            d="M18 34 32 21l14 13v11a2 2 0 0 1-2 2h-7v-9h-10v9h-7a2 2 0 0 1-2-2z"
            fill="var(--surface)"
          /><circle cx="46" cy="18" r="5" fill="var(--accent)" /></svg
        >
      </div>
      <h1>{system.title}</h1>
      <p class="lede">登录以继续</p>

      {#if methods.passkey}
        <button class="primary pressable" type="button" onclick={withPasskey} disabled={!!busy}>
          <Icon name="key" size={16} />
          {busy === "passkey" ? "等待 Passkey…" : "使用 Passkey 登录"}
        </button>
      {/if}

      {#if methods.passkey && methods.password}
        <div class="divider"><span>或使用密码</span></div>
      {/if}

      {#if methods.password}
        <form onsubmit={withPassword}>
          {#if methods.username}
            <label>
              <span>用户名</span>
              <input
                autocomplete="username"
                autocapitalize="none"
                spellcheck="false"
                bind:value={username}
                disabled={!!busy}
              />
            </label>
          {/if}
          <label>
            <span>密码</span>
            <input type="password" autocomplete="current-password" bind:value={password} disabled={!!busy} />
          </label>
          <button class="{methods.passkey ? 'secondary' : 'primary'} pressable" type="submit" disabled={!canSubmit}>
            {busy === "password" ? "正在登录…" : "登录"}
          </button>
        </form>
      {/if}

      <label class="remember">
        <input type="checkbox" bind:checked={remember} />
        <span class="box" aria-hidden="true"><Icon name="check" size={11} stroke={3} /></span>
        30 天内保持登录
      </label>

      <p class="message" role="alert" aria-live="polite">{message}</p>
    </div>
  {/key}
</main>

<style>
  .stage {
    min-height: 100dvh;
    display: grid;
    place-items: center;
    padding: 24px 16px calc(24px + env(safe-area-inset-bottom));
  }

  .panel {
    width: min(100%, 360px);
    padding: 36px 30px 18px;
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: 24px;
    box-shadow: var(--shadow-pop);
    text-align: center;
    animation: rise 420ms var(--ease-out) both;
  }
  .panel.shake {
    animation: shake 360ms var(--ease-out);
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(8px) scale(0.985);
    }
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

  .mark {
    display: inline-grid;
    color: var(--ink);
    margin-bottom: 18px;
  }

  h1 {
    margin: 0;
    font-size: 21px;
    font-weight: 600;
    letter-spacing: -0.02em;
  }
  .lede {
    margin: 4px 0 26px;
    color: var(--muted);
    font-size: 13.5px;
  }

  .primary,
  .secondary {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: 100%;
    height: 42px;
    border-radius: 12px;
    font-weight: 550;
    font-size: 14px;
  }
  .primary {
    background: var(--ink);
    color: var(--surface);
  }
  .secondary {
    background: var(--surface-2);
    border: 1px solid var(--line-strong);
  }
  .primary:disabled,
  .secondary:disabled {
    opacity: 0.5;
    cursor: default;
    transform: none;
  }

  .divider {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 20px 0 16px;
    color: var(--muted);
    font-size: 12px;
  }
  .divider::before,
  .divider::after {
    content: "";
    flex: 1;
    height: 1px;
    background: var(--line-strong);
  }

  form {
    display: grid;
    gap: 12px;
    text-align: left;
  }
  form label {
    display: grid;
    gap: 6px;
  }
  form label span {
    font-size: 12.5px;
    font-weight: 500;
    color: var(--ink-2);
  }
  input:not([type="checkbox"]) {
    height: 40px;
    padding: 0 12px;
    border-radius: 11px;
    border: 1px solid var(--line-strong);
    background: var(--surface-2);
    font-size: 16px; /* prevents iOS zoom on focus */
    outline: none;
    transition:
      border-color 160ms ease,
      box-shadow 160ms ease;
  }
  input:not([type="checkbox"]):focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  form button {
    margin-top: 4px;
  }

  .remember {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    margin-top: 18px;
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
    border-radius: 5px;
    border: 1px solid var(--line-strong);
    background: var(--surface-2);
    color: transparent;
    transition:
      background-color 140ms ease,
      border-color 140ms ease,
      color 140ms ease;
  }
  .remember input:checked + .box {
    background: var(--ink);
    border-color: var(--ink);
    color: var(--surface);
  }
  .remember input:focus-visible + .box {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .message {
    min-height: 20px;
    margin: 12px 0 0;
    font-size: 13px;
    color: var(--danger);
  }
</style>

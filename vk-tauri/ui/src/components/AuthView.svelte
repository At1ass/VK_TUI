<script>
  import { invoke } from '@tauri-apps/api/core';

  export let externalError = null;
  export let onLogin;

  let redirectUrl = '';
  let loading = false;
  let localError = null;

  async function openAuthUrl() {
    try {
      const url = await invoke('get_auth_url');
      console.log('Opening URL:', url);

      // Try using shell plugin
      try {
        const { open } = await import('@tauri-apps/plugin-shell');
        await open(url);
      } catch (shellError) {
        console.error('Shell plugin error:', shellError);
        // Fallback: copy URL to clipboard and show message
        error = `Скопируйте URL: ${url}`;
      }
    } catch (e) {
      console.error('Failed to open auth URL:', e);
      localError = `Ошибка: ${e}`;
    }
  }

  async function handleSubmit() {
    if (!redirectUrl.trim()) {
      localError = 'Введите redirect URL';
      return;
    }

    loading = true;
    await onLogin(redirectUrl);
    loading = false;
  }
</script>

<div class="auth-container">
  <div class="auth-card">
    <h1>VK Messenger</h1>

    {#if localError || externalError}
      <div class="error">
        {localError || externalError}
      </div>
    {/if}

    <p class="hint">
      Авторизуйтесь через браузер, затем вставьте redirect URL
    </p>

    <div class="form">
      <input
        type="text"
        placeholder="Вставьте redirect URL..."
        bind:value={redirectUrl}
        on:keypress={(e) => e.key === 'Enter' && handleSubmit()}
        disabled={loading}
      />

      <div class="button-row">
        <button class="button" on:click={openAuthUrl} disabled={loading}>
          Открыть OAuth
        </button>
        <button class="button suggested" on:click={handleSubmit} disabled={loading}>
          {loading ? 'Вход...' : 'Войти'}
        </button>
      </div>
    </div>

    <p class="help">
      После авторизации в браузере скопируйте полный URL из адресной строки
    </p>
  </div>
</div>

<style>
  .auth-container {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100vh;
    background: var(--window-bg-color);
  }

  .auth-card {
    background: var(--card-bg-color);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-l);
    padding: 2rem;
    width: 90%;
    max-width: 500px;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.25);
  }

  h1 {
    font-size: 20px;
    font-weight: 600;
    margin-bottom: 1rem;
    text-align: center;
    color: var(--window-fg-color);
  }

  .hint {
    color: var(--muted-fg-color);
    text-align: center;
    margin-bottom: 1.5rem;
    font-size: 14px;
  }

  .error {
    background: rgba(255, 122, 122, 0.1);
    border: 1px solid var(--destructive-bg-color);
    border-radius: var(--radius-m);
    padding: 0.75rem;
    margin-bottom: 1rem;
    color: var(--destructive-bg-color);
    text-align: center;
  }

  .form {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  input {
    background: var(--entry-bg-color);
    border: 1px solid var(--entry-border-color);
    border-radius: var(--radius-s);
    padding: 0.75rem 1rem;
    color: var(--view-fg-color);
    font-size: 14px;
    transition: border-color 0.2s;
  }

  input:focus {
    border-color: var(--accent-bg-color);
  }

  input:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .button-row {
    display: flex;
    gap: 0.75rem;
  }

  .button-row :global(.button) {
    flex: 1;
    padding: 0.65rem 1.2rem;
    font-weight: 600;
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .help {
    margin-top: 1.5rem;
    font-size: 12px;
    color: var(--muted-fg-color);
    text-align: center;
  }
</style>

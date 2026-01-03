<script>
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-shell';

  export let error = null;
  export let onLogin;

  let redirectUrl = '';
  let loading = false;

  async function openAuthUrl() {
    try {
      const url = await invoke('get_auth_url');
      await open(url);
    } catch (e) {
      console.error('Failed to open auth URL:', e);
    }
  }

  async function handleSubmit() {
    if (!redirectUrl.trim()) {
      error = 'Введите redirect URL';
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

    {#if error}
      <div class="error">
        {error}
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
        <button class="btn-secondary" on:click={openAuthUrl} disabled={loading}>
          Открыть OAuth
        </button>
        <button class="btn-primary" on:click={handleSubmit} disabled={loading}>
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
    background: var(--cosmic-bg);
  }

  .auth-card {
    background: var(--cosmic-surface);
    border: 1px solid var(--cosmic-border);
    border-radius: 12px;
    padding: 2rem;
    width: 90%;
    max-width: 500px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  }

  h1 {
    font-size: 32px;
    font-weight: 600;
    margin-bottom: 1rem;
    text-align: center;
    color: var(--cosmic-text);
  }

  .hint {
    color: var(--cosmic-muted);
    text-align: center;
    margin-bottom: 1.5rem;
    font-size: 14px;
  }

  .error {
    background: rgba(255, 122, 122, 0.1);
    border: 1px solid var(--cosmic-danger);
    border-radius: 8px;
    padding: 0.75rem;
    margin-bottom: 1rem;
    color: var(--cosmic-danger);
    text-align: center;
  }

  .form {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  input {
    background: var(--cosmic-surface-alt);
    border: 1px solid var(--cosmic-border);
    border-radius: 8px;
    padding: 0.75rem 1rem;
    color: var(--cosmic-text);
    font-size: 14px;
    transition: border-color 0.2s;
  }

  input:focus {
    border-color: var(--cosmic-accent);
  }

  input:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .button-row {
    display: flex;
    gap: 0.75rem;
  }

  button {
    padding: 0.75rem 1.5rem;
    border-radius: 8px;
    font-weight: 600;
    transition: all 0.2s;
    flex: 1;
  }

  .btn-primary {
    background: var(--cosmic-accent);
    color: var(--cosmic-bg);
  }

  .btn-primary:hover:not(:disabled) {
    background: #6db9ff;
    box-shadow: 0 2px 8px rgba(88, 170, 255, 0.3);
  }

  .btn-secondary {
    background: var(--cosmic-surface-alt);
    border: 1px solid var(--cosmic-border);
    color: var(--cosmic-text);
  }

  .btn-secondary:hover:not(:disabled) {
    background: #202638;
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .help {
    margin-top: 1.5rem;
    font-size: 12px;
    color: var(--cosmic-muted);
    text-align: center;
  }
</style>

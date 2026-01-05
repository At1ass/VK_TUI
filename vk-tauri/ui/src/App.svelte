<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import {
    isPermissionGranted,
    requestPermission,
  } from '@tauri-apps/plugin-notification';
  import AuthView from './components/AuthView.svelte';
  import MainView from './components/MainView.svelte';

  let authenticated = false;
  let loading = true;
  let error = null;

  onMount(async () => {
    try {
      // Check if already authenticated
      authenticated = await invoke('is_authenticated');

      if (authenticated) {
        // Validate session
        await invoke('validate_session');
      }
    } catch (e) {
      console.error('Session validation failed:', e);
      authenticated = false;
    } finally {
      loading = false;
    }

    // Request notification permissions (separately, so it doesn't break auth)
    try {
      let permissionGranted = await isPermissionGranted();
      if (!permissionGranted) {
        const permission = await requestPermission();
        permissionGranted = permission === 'granted';
      }
    } catch (e) {
      console.warn('Notification permissions request failed:', e);
      // Non-critical, continue without notifications
    }
  });

  async function handleLogin(redirectUrl) {
    try {
      error = null;
      loading = true;
      await invoke('login', { redirectUrl });
      authenticated = true;
    } catch (e) {
      error = e;
      console.error('Login failed:', e);
    } finally {
      loading = false;
    }
  }

  async function handleLogout() {
    try {
      await invoke('logout');
      authenticated = false;
    } catch (e) {
      console.error('Logout failed:', e);
    }
  }
</script>

<main>
  {#if loading}
    <div class="loading">
      <div class="spinner"></div>
      <p>Загрузка...</p>
    </div>
  {:else if !authenticated}
    <AuthView externalError={error} onLogin={handleLogin} />
  {:else}
    <MainView onLogout={handleLogout} />
  {/if}
</main>

<style>
  main {
    width: 100%;
    height: 100vh;
    overflow: hidden;
  }

  .loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100vh;
    gap: 1rem;
  }

  .spinner {
    width: 40px;
    height: 40px;
    border: 4px solid var(--card-bg-color);
    border-top-color: var(--accent-bg-color);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>

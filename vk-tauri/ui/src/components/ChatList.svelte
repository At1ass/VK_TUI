<script>
  import { onMount, onDestroy } from 'svelte';

  export let chats = [];
  export let loading = false;
  export let selectedChatId = null;
  export let onSelectChat;
  export let hasMore = false;
  export let loadingMore = false;
  export let onLoadMore;

  let focusedIndex = -1;
  let chatListEl = null;

  onMount(() => {
    const handleKeyDown = (e) => {
      if (chats.length === 0) return;

      // Arrow Down - move focus down
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        focusedIndex = Math.min(focusedIndex + 1, chats.length - 1);
        // If we were at -1, start from selected or 0
        if (focusedIndex === -1) {
          const selectedIndex = chats.findIndex(c => c.id === selectedChatId);
          focusedIndex = selectedIndex >= 0 ? selectedIndex : 0;
        }
      }
      // Arrow Up - move focus up
      else if (e.key === 'ArrowUp') {
        e.preventDefault();
        if (focusedIndex <= 0) {
          focusedIndex = 0;
        } else {
          focusedIndex = Math.max(0, focusedIndex - 1);
        }
      }
      // Enter - select focused chat
      else if (e.key === 'Enter' && focusedIndex >= 0 && focusedIndex < chats.length) {
        e.preventDefault();
        onSelectChat(chats[focusedIndex]);
      }
    };

    const handleScroll = () => {
      if (!chatListEl || !hasMore || loadingMore) return;

      const { scrollTop, scrollHeight, clientHeight } = chatListEl;
      const threshold = 200; // pixels from bottom

      if (scrollTop + clientHeight >= scrollHeight - threshold) {
        onLoadMore?.();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    if (chatListEl) {
      chatListEl.addEventListener('scroll', handleScroll);
    }

    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      if (chatListEl) {
        chatListEl.removeEventListener('scroll', handleScroll);
      }
    };
  });

  onDestroy(() => {
    // Cleanup is handled by onMount return
  });

  function truncate(text, maxLen = 30) {
    if (text.length <= maxLen) return text;
    return text.substring(0, maxLen) + '...';
  }

  $: if (selectedChatId) {
    const selectedIndex = chats.findIndex(c => c.id === selectedChatId);
    if (selectedIndex >= 0) {
      focusedIndex = selectedIndex;
    }
  }
</script>

<div class="chat-list" bind:this={chatListEl}>
  {#if loading}
    <div class="loading">
      <div class="spinner"></div>
      <p>Загрузка чатов...</p>
    </div>
  {:else if chats.length === 0}
    <div class="empty">
      <p>Нет чатов</p>
    </div>
  {:else}
    {#each chats as chat, i (chat.id)}
      <button
        class="chat-item"
        class:selected={chat.id === selectedChatId}
        class:focused={i === focusedIndex}
        on:click={() => onSelectChat(chat)}
      >
        <div class="chat-header">
          <span class="chat-title">{chat.title}</span>
          {#if chat.is_online}
            <span class="online-indicator">●</span>
          {/if}
          {#if chat.unread_count > 0}
            <span class="unread-badge">{chat.unread_count}</span>
          {/if}
        </div>
        <p class="chat-preview">{truncate(chat.last_message)}</p>
      </button>
    {/each}

    {#if loadingMore}
      <div class="loading-more">
        <div class="spinner-small"></div>
        <span>Загрузка...</span>
      </div>
    {/if}
  {/if}
</div>

<style>
  /* Sidebar - GNOME HIG compliant */
  .chat-list {
    min-width: 270px;
    max-width: 420px;
    width: 100%;
    height: 100%;
    background: var(--sidebar-bg-color);
    border-right: 1px solid var(--sidebar-shade-color);
    box-shadow: inset -1px 0 var(--sidebar-shade-color);
    overflow-y: auto;
    flex-shrink: 0;
    padding: 0;
  }

  /* Responsive Breakpoints - GNOME HIG */

  /* Desktop: > 900px - Full layout (default) */
  @media (min-width: 900px) {
    .chat-list {
      min-width: 270px;
      max-width: 420px;
    }
  }

  /* Tablet: 600px - 900px - Narrow sidebar */
  @media (min-width: 600px) and (max-width: 900px) {
    .chat-list {
      min-width: 200px;
      max-width: 280px;
    }
  }

  /* Mobile: < 600px - Full width when revealed */
  @media (max-width: 600px) {
    .chat-list {
      width: 100%;
      min-width: 100%;
      max-width: 100%;
    }
  }

  .loading, .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 200px;
    gap: 1rem;
    color: var(--muted-fg-color);
  }

  .spinner {
    width: 30px;
    height: 30px;
    border: 3px solid var(--card-bg-color);
    border-top-color: var(--accent-bg-color);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* List Row - GNOME HIG compliant */
  .chat-item {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 12px;
    min-height: 56px;
    width: 100%;
    text-align: left;
    background: transparent;
    border: none;
    border-radius: 0;
    margin: 0;
    transition: background 150ms ease-out;
  }

  .chat-item:hover {
    background: var(--row-hover-bg-color);
  }

  .chat-item.selected {
    background: var(--accent-bg-color);
  }

  .chat-item.selected .chat-title {
    color: var(--accent-fg-color);
  }

  .chat-item.selected .chat-preview {
    color: rgba(255, 255, 255, 0.7);
  }

  .chat-item.focused:not(.selected) {
    background: var(--row-hover-bg-color);
    outline: 2px solid var(--accent-bg-color);
    outline-offset: -2px;
  }

  .chat-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .chat-title {
    font-weight: 600;
    font-size: 13px;
    flex: 1;
    line-height: 1.2;
  }

  .online-indicator {
    color: var(--success-bg-color);
    font-size: 8px;
    line-height: 1;
  }

  .unread-badge {
    background: var(--accent-bg-color);
    color: var(--accent-fg-color);
    font-size: 10px;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: 999px;
    min-width: 20px;
    text-align: center;
    line-height: 1.2;
  }

  .chat-preview {
    font-size: 11px;
    color: var(--muted-fg-color);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    line-height: 1.3;
  }

  .loading-more {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 1rem;
    color: var(--muted-fg-color);
    font-size: 11px;
  }

  .spinner-small {
    width: 16px;
    height: 16px;
    border: 2px solid var(--card-bg-color);
    border-top-color: var(--accent-bg-color);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
</style>

<script>
  export let chats = [];
  export let loading = false;
  export let selectedChatId = null;
  export let onSelectChat;

  function truncate(text, maxLen = 30) {
    if (text.length <= maxLen) return text;
    return text.substring(0, maxLen) + '...';
  }
</script>

<div class="chat-list">
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
    {#each chats as chat (chat.id)}
      <button
        class="chat-item"
        class:selected={chat.id === selectedChatId}
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
  {/if}
</div>

<style>
  .chat-list {
    width: 300px;
    height: 100%;
    background: var(--cosmic-surface);
    border-right: 1px solid var(--cosmic-border);
    overflow-y: auto;
    flex-shrink: 0;
  }

  .loading, .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 200px;
    gap: 1rem;
    color: var(--cosmic-muted);
  }

  .spinner {
    width: 30px;
    height: 30px;
    border: 3px solid var(--cosmic-surface-alt);
    border-top-color: var(--cosmic-accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .chat-item {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    width: 100%;
    text-align: left;
    background: var(--cosmic-surface);
    border-bottom: 1px solid var(--cosmic-border);
    transition: background 0.2s;
  }

  .chat-item:hover {
    background: #1e2536;
  }

  .chat-item.selected {
    background: var(--cosmic-surface-alt);
    border-left: 3px solid var(--cosmic-accent);
  }

  .chat-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .chat-title {
    font-weight: 600;
    font-size: 14px;
    flex: 1;
  }

  .online-indicator {
    color: var(--cosmic-success);
    font-size: 10px;
  }

  .unread-badge {
    background: var(--cosmic-accent);
    color: var(--cosmic-bg);
    font-size: 11px;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: 10px;
  }

  .chat-preview {
    font-size: 12px;
    color: var(--cosmic-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>

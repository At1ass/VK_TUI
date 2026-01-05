<script>
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import ChatList from './ChatList.svelte';
  import MessageView from './MessageView.svelte';

  export let onLogout;

  let chats = [];
  let messages = [];
  let users = {};
  let selectedChat = null;
  let loading = false;
  let status = 'Подключение...';
  let typingTimeoutId = null;
  let unlistenCore = null;
  let searchQuery = '';
  let searchResults = [];
  let searchTotal = 0;
  let searchLoading = false;
  let searchOpen = false;
  let searchInChat = false;
  let paginationMode = 'latest';
  let pendingLoadDirection = 'replace';
  let loadingMore = false;
  let hasMoreOlder = true;
  let hasMoreNewer = false;
  let lastLoadKey = null;
  let lastLoadAt = 0;
  let paginationAnchorId = null;
  let paginationOffset = 0;
  let searchBarVisible = false;
  let sidebarRevealed = false;

  onMount(async () => {
    try {
      // Load conversations
      loading = true;
      await invoke('load_conversations', { offset: 0 });

      unlistenCore = await listen('core:event', (event) => {
        handleEvent(event.payload);
      });
    } catch (e) {
      console.error('Failed to load conversations:', e);
      status = `Ошибка: ${e}`;
    }
  });

  onDestroy(() => {
    if (unlistenCore) {
      unlistenCore();
    }
  });

  function handleEvent(event) {
    if (event.ConversationsLoaded) {
      const { chats: newChats, profiles } = event.ConversationsLoaded;
      chats = newChats;

      // Update users map
      for (const profile of profiles) {
        users[profile.id] = profile;
      }

      loading = false;
      status = 'Готово';
    } else if (event.MessagesLoaded) {
      const { peer_id, messages: newMessages, profiles } = event.MessagesLoaded;

      if (selectedChat && selectedChat.id === peer_id) {
        let addedCount = 0;
        if (pendingLoadDirection === 'older') {
          const merged = mergeOlder(messages, newMessages);
          messages = merged.items;
          addedCount = merged.added;
          hasMoreOlder = merged.added > 0 && (event.MessagesLoaded.has_more ?? false);
          if (merged.added === 0) {
            hasMoreOlder = false;
          }
        } else if (pendingLoadDirection === 'newer') {
          const merged = mergeNewer(messages, newMessages);
          messages = merged.items;
          addedCount = merged.added;
          hasMoreNewer = merged.added > 0 && (event.MessagesLoaded.has_more ?? false);
          if (merged.added === 0) {
            hasMoreNewer = false;
          }
        } else {
          messages = newMessages;
          hasMoreOlder = event.MessagesLoaded.has_more ?? false;
          hasMoreNewer = paginationMode === 'around';
        }

        if (pendingLoadDirection === 'older') {
          paginationOffset += addedCount;
        } else {
          updatePaginationAnchor(messages);
        }
        pendingLoadDirection = 'replace';
        loadingMore = false;

        // Update users
        for (const profile of profiles) {
          users[profile.id] = profile;
        }
      }
    } else if (event.SearchResultsLoaded) {
      searchResults = event.SearchResultsLoaded.results || [];
      searchTotal = event.SearchResultsLoaded.total_count || 0;
      searchLoading = false;
      searchOpen = true;
    } else if (event.VkEvent) {
      handleVkEvent(event.VkEvent);
    } else if (event.MessageSent) {
      if (selectedChat) {
        invoke('load_messages', { peerId: selectedChat.id, offset: 0 }).catch(() => {});
      }
    } else if (event.MessageEdited) {
      const { message_id } = event.MessageEdited;
      invoke('fetch_message_by_id', { messageId: message_id }).catch(() => {});
    } else if (event.MessageDeleted) {
      const { message_id } = event.MessageDeleted;
      messages = messages.filter(m => m.id !== message_id);
    } else if (event.MessageDetailsFetched) {
      const { message_id } = event.MessageDetailsFetched;
      const idx = messages.findIndex(m => m.id === message_id);
      if (idx !== -1) {
        const current = messages[idx];
        messages[idx] = {
          ...current,
          cmid: event.MessageDetailsFetched.cmid ?? current.cmid,
          text: event.MessageDetailsFetched.text ?? current.text,
          is_edited: event.MessageDetailsFetched.is_edited ?? current.is_edited,
          attachments: event.MessageDetailsFetched.attachments ?? current.attachments,
          reply: event.MessageDetailsFetched.reply ?? current.reply,
          fwd_count: event.MessageDetailsFetched.fwd_count ?? current.fwd_count,
          forwards: event.MessageDetailsFetched.forwards ?? current.forwards,
        };
        messages = messages;
      }
    } else if (event.SendFailed) {
      status = `Ошибка: ${event.SendFailed}`;
    } else if (event.Error) {
      status = `Ошибка: ${event.Error}`;
      loadingMore = false;
      pendingLoadDirection = 'replace';
      console.error('Core error:', event.Error);
    }
  }

  function handleVkEvent(vkEvent) {
    if (vkEvent.NewMessage) {
      const { message_id, peer_id, text, from_id, timestamp } = vkEvent.NewMessage;

      // Update unread counter for chat
      const chat = chats.find(c => c.id === peer_id);
      if (chat) {
        if (!selectedChat || selectedChat.id !== peer_id) {
          chat.unread_count = (chat.unread_count || 0) + 1;
        } else {
          chat.unread_count = 0;
        }
        chat.last_message = text;
        chats = chats; // Trigger reactivity
      }

      // Add to messages if chat is selected
      if (selectedChat && selectedChat.id === peer_id) {
        messages = [...messages, {
          id: message_id,
          from_id,
          from_name: getUserName(from_id),
          text,
          timestamp: timestamp || Math.floor(Date.now() / 1000),
          is_outgoing: false,
          is_read: true,
          is_edited: false,
          attachments: [],
        }];
        invoke('mark_as_read', { peerId: peer_id }).catch(() => {});
      }
    } else if (vkEvent.ConnectionStatus !== undefined) {
      status = vkEvent.ConnectionStatus ? 'Подключено' : 'Отключено';
    } else if (vkEvent.UserTyping) {
      const { peer_id, user_id } = vkEvent.UserTyping;
      if (selectedChat && selectedChat.id === peer_id) {
        status = `${getUserName(user_id)} печатает...`;
        if (typingTimeoutId) {
          clearTimeout(typingTimeoutId);
        }
        typingTimeoutId = setTimeout(() => {
          status = 'Готово';
          typingTimeoutId = null;
        }, 3000);
      }
    } else if (vkEvent.MessageEditedFromLongPoll) {
      const { peer_id, message_id } = vkEvent.MessageEditedFromLongPoll;
      if (selectedChat && selectedChat.id === peer_id) {
        invoke('fetch_message_by_id', { messageId: message_id }).catch(() => {});
      }
    } else if (vkEvent.MessageDeletedFromLongPoll) {
      const { peer_id, message_id } = vkEvent.MessageDeletedFromLongPoll;
      if (selectedChat && selectedChat.id === peer_id) {
        messages = messages.filter(m => m.id !== message_id);
      }
    } else if (vkEvent.MessageRead) {
      const { peer_id, message_id } = vkEvent.MessageRead;
      if (selectedChat && selectedChat.id === peer_id) {
        messages = messages.map(m => {
          if (m.is_outgoing && (message_id <= 0 || m.id <= message_id)) {
            return { ...m, is_read: true };
          }
          return m;
        });
      }
      const chat = chats.find(c => c.id === peer_id);
      if (chat) {
        chat.unread_count = 0;
        chats = chats;
      }
    }
  }

  function getUserName(userId) {
    const user = users[userId];
    if (user) {
      return `${user.first_name} ${user.last_name}`;
    }
    return `User ${userId}`;
  }

  async function handleChatSelect(chat) {
    selectedChat = chat;
    messages = [];
    paginationMode = 'latest';
    hasMoreOlder = true;
    hasMoreNewer = false;
    pendingLoadDirection = 'replace';
    paginationAnchorId = null;
    paginationOffset = 0;
    if (typingTimeoutId) {
      clearTimeout(typingTimeoutId);
      typingTimeoutId = null;
    }
    status = 'Готово';

    try {
      await invoke('load_messages', { peerId: chat.id, offset: 0 });
      await invoke('mark_as_read', { peerId: chat.id });
      const target = chats.find(c => c.id === chat.id);
      if (target) {
        target.unread_count = 0;
        chats = chats;
      }
    } catch (e) {
      console.error('Failed to load messages:', e);
    }
  }

  function toggleSearchBar() {
    searchBarVisible = !searchBarVisible;
    if (!searchBarVisible) {
      searchQuery = '';
      searchOpen = false;
    }
  }

  function toggleSidebar() {
    sidebarRevealed = !sidebarRevealed;
  }

  function closeSidebar() {
    sidebarRevealed = false;
  }

  async function handleSearch() {
    const query = searchQuery.trim();
    if (!query) return;

    searchLoading = true;
    searchOpen = true;
    searchResults = [];
    searchTotal = 0;

    const peerId = searchInChat && selectedChat ? selectedChat.id : null;
    try {
      await invoke('search_messages', { query, peerId });
    } catch (e) {
      console.error('Search failed:', e);
      searchLoading = false;
    }
  }

  async function handleSearchResultClick(result) {
    const chatId = result.peer_id;
    const messageId = result.message_id;

    let chat = chats.find(c => c.id === chatId);
    if (!chat) {
      chat = {
        id: chatId,
        title: result.chat_title || `Chat ${chatId}`,
        last_message: result.text,
        last_message_time: result.timestamp,
        unread_count: 0,
        is_online: false,
      };
      chats = [chat, ...chats];
    }

    selectedChat = chat;
    messages = [];
    searchOpen = false;
    paginationMode = 'around';
    hasMoreOlder = true;
    hasMoreNewer = true;
    pendingLoadDirection = 'replace';
    paginationAnchorId = null;
    paginationOffset = 0;

    try {
      await invoke('load_messages_around', { peerId: chatId, messageId });
    } catch (e) {
      console.error('Failed to load messages around search result:', e);
    }
  }

  async function handleLoadMore(direction, anchor) {
    if (!selectedChat || !anchor || loadingMore) return;
    if (!anchor.id) return;
    if (direction === 'older' && !hasMoreOlder) return;
    if (direction === 'newer' && !hasMoreNewer) return;

    const loadKey = `${direction}:${anchor.id}:${paginationOffset}`;
    const now = Date.now();
    if (lastLoadKey === loadKey && now - lastLoadAt < 1500) {
      return;
    }
    lastLoadKey = loadKey;
    lastLoadAt = now;

    loadingMore = true;
    pendingLoadDirection = direction;

    const count = 50;

    try {
      if (direction === 'older') {
        const startMessageId = paginationAnchorId ?? anchor.id;
        await invoke('load_messages_with_start_message_id', {
          peerId: selectedChat.id,
          startMessageId,
          offset: paginationOffset,
          count,
        });
      } else if (paginationMode === 'around') {
        await invoke('load_messages', { peerId: selectedChat.id, offset: 0 });
      }
    } catch (e) {
      console.error('Failed to load more messages:', e);
      loadingMore = false;
    }
  }

  function mergeOlder(existing, incoming) {
    const existingIds = new Set(existing.map(m => m.id));
    const filtered = incoming.filter(m => !existingIds.has(m.id));
    return { items: [...filtered, ...existing], added: filtered.length };
  }

  function mergeNewer(existing, incoming) {
    const existingIds = new Set(existing.map(m => m.id));
    const filtered = incoming.filter(m => !existingIds.has(m.id));
    return { items: [...existing, ...filtered], added: filtered.length };
  }

  function updatePaginationAnchor(list) {
    if (!list.length) {
      paginationAnchorId = null;
      paginationOffset = 0;
      return;
    }
    let maxId = list[0].id;
    for (const msg of list) {
      if (msg.id > maxId) {
        maxId = msg.id;
      }
    }
    paginationAnchorId = maxId;
    paginationOffset = list.length;
  }

  async function handleSendMessage(text) {
    if (!selectedChat || !text.trim()) return;

    try {
      await invoke('send_message', { peerId: selectedChat.id, text });
    } catch (e) {
      console.error('Failed to send message:', e);
    }
  }

  async function handleSendReply(replyTo, text) {
    if (!selectedChat || !text.trim()) return;

    try {
      await invoke('send_reply', {
        peerId: selectedChat.id,
        replyTo,
        text,
      });
    } catch (e) {
      console.error('Failed to send reply:', e);
    }
  }

  async function handleForward(messageIds, comment) {
    if (!selectedChat || !comment.trim() || messageIds.length === 0) return;

    try {
      await invoke('send_forward', {
        peerId: selectedChat.id,
        messageIds,
        comment,
      });
    } catch (e) {
      console.error('Failed to forward messages:', e);
    }
  }

  async function handleEditMessage(messageId, cmid, text) {
    if (!selectedChat || !text.trim()) return;

    try {
      await invoke('edit_message', {
        peerId: selectedChat.id,
        messageId,
        cmid: cmid ?? null,
        text,
      });
    } catch (e) {
      console.error('Failed to edit message:', e);
    }
  }

  async function handleDeleteMessages(messageIds, forAll) {
    if (!selectedChat || messageIds.length === 0) return;

    for (const messageId of messageIds) {
      try {
        await invoke('delete_message', {
          peerId: selectedChat.id,
          messageId,
          forAll,
        });
      } catch (e) {
        console.error('Failed to delete message:', e);
      }
    }
  }
</script>

<div class="main-view">
  <header class="headerbar" data-tauri-drag-region>
    <div class="headerbar-start">
      <button
        class="button flat icon-button sidebar-toggle"
        on:click={toggleSidebar}
        title="Показать чаты"
        aria-label="Показать чаты"
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
          <path d="M2 3h12v2H2V3zm0 4h12v2H2V7zm0 4h12v2H2v-2z"/>
        </svg>
      </button>
    </div>

    <div class="headerbar-center" data-tauri-drag-region>
      <h1 class="headerbar-title">Сообщения</h1>
      {#if status && status !== 'Готово'}
        <span class="headerbar-subtitle">{status}</span>
      {/if}
    </div>

    <div class="headerbar-end">
      <button
        class="button flat icon-button"
        on:click={toggleSearchBar}
        title="Поиск (Ctrl+F)"
        aria-label="Поиск"
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
          <path d="M6.5 1C3.46 1 1 3.46 1 6.5S3.46 12 6.5 12c1.41 0 2.69-.53 3.66-1.41l3.63 3.63 1.41-1.41-3.63-3.63C12.47 8.19 13 6.91 13 5.5 13 2.46 10.54 0 7.5 0zm0 2c2.21 0 4 1.79 4 4s-1.79 4-4 4-4-1.79-4-4 1.79-4 4-4z"/>
        </svg>
      </button>
      <button
        class="button flat icon-button"
        on:click={onLogout}
        title="Выйти"
        aria-label="Выйти"
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
          <path d="M3 2v12h5v-2H5V4h3V2H3zm7 2l-1.5 1.5L11 8H6v2h5l-2.5 2.5L10 14l5-5-5-5z"/>
        </svg>
      </button>
    </div>
  </header>

  {#if searchBarVisible}
    <div class="search-bar">
      <input
        type="search"
        class="search-input"
        placeholder="Поиск сообщений..."
        bind:value={searchQuery}
        on:keypress={(e) => e.key === 'Enter' && handleSearch()}
      />
      <label class="search-scope">
        <input type="checkbox" bind:checked={searchInChat} disabled={!selectedChat} />
        В текущем чате
      </label>
      <button class="button suggested" on:click={handleSearch}>
        Найти
      </button>
      <button class="button flat" on:click={toggleSearchBar}>
        Закрыть
      </button>
    </div>
  {/if}

  <div class="content">
    <div class="sidebar-container" class:revealed={sidebarRevealed}>
      <ChatList
        {chats}
        {loading}
        selectedChatId={selectedChat?.id}
        onSelectChat={(chat) => {
          handleChatSelect(chat);
          closeSidebar();
        }}
      />
    </div>

    {#if sidebarRevealed}
      <div class="sidebar-overlay" on:click={closeSidebar}></div>
    {/if}

    {#if selectedChat}
      <MessageView
        chat={selectedChat}
        {messages}
        {users}
        onSendMessage={handleSendMessage}
        onSendReply={handleSendReply}
        onForward={handleForward}
        onEditMessage={handleEditMessage}
        onDeleteMessages={handleDeleteMessages}
        onLoadMore={handleLoadMore}
        canLoadOlder={hasMoreOlder}
        canLoadNewer={hasMoreNewer}
        autoScroll={paginationMode === 'latest'}
      />
    {:else}
      <div class="empty-state">
        <p>Выберите чат</p>
      </div>
    {/if}
  </div>

  {#if searchOpen}
    <div class="search-panel">
      <div class="search-panel-header">
        <span>Результаты поиска ({searchTotal})</span>
        <button class="button flat" on:click={() => (searchOpen = false)}>Закрыть</button>
      </div>
      {#if searchLoading}
        <div class="search-state">Поиск...</div>
      {:else if searchResults.length === 0}
        <div class="search-state">Ничего не найдено</div>
      {:else}
        <div class="search-results">
          {#each searchResults as result (result.message_id)}
            <button class="search-result" on:click={() => handleSearchResultClick(result)}>
              <div class="search-title">
                {result.chat_title}
              </div>
              <div class="search-meta">
                <span>{result.from_name}</span>
                <span>{new Date(result.timestamp * 1000).toLocaleString('ru-RU')}</span>
              </div>
              <div class="search-text">
                {result.text}
              </div>
            </button>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .main-view {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100vh;
    background: var(--window-bg-color);
  }

  /* Header Bar - GNOME HIG compliant */
  .headerbar {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 6px;
    padding: 0 6px;
    min-height: 46px;
    background: var(--headerbar-bg-color);
    border-bottom: 1px solid var(--headerbar-border-color);
    box-shadow: 0 1px var(--headerbar-shade-color);
  }

  .headerbar-start,
  .headerbar-end {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .headerbar-center {
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    gap: 0;
  }

  .headerbar-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--headerbar-fg-color);
    margin: 0;
  }

  .headerbar-subtitle {
    font-size: 11px;
    color: var(--muted-fg-color);
  }

  /* Search Bar - separate from header */
  .search-bar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 6px 12px;
    background: var(--headerbar-bg-color);
    border-bottom: 1px solid var(--border-color);
  }

  .search-input {
    flex: 1;
    min-width: 0;
  }

  .search-scope {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--view-fg-color);
    white-space: nowrap;
  }

  .content {
    display: flex;
    flex: 1;
    overflow: hidden;
    position: relative;
  }

  .sidebar-container {
    display: flex;
    flex-shrink: 0;
    width: 25%;
    min-width: 270px;
    max-width: 420px;
  }

  .sidebar-overlay {
    display: none;
  }

  /* Mobile toggle button - hidden on desktop */
  .sidebar-toggle {
    display: none;
  }

  /* Responsive Breakpoints - GNOME HIG */

  /* Mobile: < 600px - Collapsed sidebar */
  @media (max-width: 600px) {
    .sidebar-toggle {
      display: inline-flex;
    }

    .sidebar-container {
      position: fixed;
      left: -100%;
      top: 0;
      bottom: 0;
      width: 80%;
      min-width: unset;
      max-width: 320px;
      z-index: 200;
      transition: left 200ms ease-out;
      box-shadow: none;
    }

    .sidebar-container.revealed {
      left: 0;
      box-shadow: 2px 0 8px rgba(0, 0, 0, 0.3);
    }

    .sidebar-overlay {
      display: block;
      position: fixed;
      inset: 0;
      background: rgba(0, 0, 0, 0.5);
      z-index: 199;
    }
  }

  /* Tablet: 600px - 900px - Narrow sidebar */
  @media (min-width: 600px) and (max-width: 900px) {
    .sidebar-container {
      width: 30%;
      min-width: 200px;
      max-width: 280px;
    }
  }

  /* Desktop: > 900px - Full layout (default) */
  @media (min-width: 900px) {
    .sidebar-container {
      width: 25%;
      min-width: 270px;
      max-width: 420px;
    }
  }

  .empty-state {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--muted-fg-color);
  }

  .search-panel {
    position: fixed;
    right: 1rem;
    top: 4.25rem;
    width: min(360px, 90vw);
    max-height: 70vh;
    overflow: hidden;
    background: var(--card-bg-color);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-l);
    display: flex;
    flex-direction: column;
    z-index: 1200;
    box-shadow: 0 8px 20px rgba(0, 0, 0, 0.35);
  }

  .search-panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--border-color);
    font-size: 12px;
    color: var(--muted-fg-color);
  }

  .search-state {
    padding: 1rem;
    text-align: center;
    color: var(--muted-fg-color);
  }

  .search-results {
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: 0.5rem;
  }

  .search-result {
    text-align: left;
    padding: 0.55rem 0.7rem;
    border-radius: var(--radius-s);
    background: transparent;
    border: 1px solid transparent;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }

  .search-result:hover {
    background: var(--row-hover-bg-color);
    border-color: var(--border-color);
  }

  .search-title {
    font-weight: 600;
    font-size: 12px;
  }

  .search-meta {
    display: flex;
    justify-content: space-between;
    font-size: 10px;
    color: var(--muted-fg-color);
  }

  .search-text {
    font-size: 12px;
    color: var(--view-fg-color);
  }
</style>

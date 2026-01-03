<script>
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import ChatList from './ChatList.svelte';
  import MessageView from './MessageView.svelte';

  export let onLogout;

  let chats = [];
  let messages = [];
  let users = {};
  let selectedChat = null;
  let loading = false;
  let status = 'Подключение...';
  let pollInterval;

  onMount(async () => {
    try {
      // Load conversations
      loading = true;
      await invoke('load_conversations', { offset: 0 });

      // Start polling for events
      pollInterval = setInterval(pollEvents, 200);
    } catch (e) {
      console.error('Failed to load conversations:', e);
      status = `Ошибка: ${e}`;
    }
  });

  onDestroy(() => {
    if (pollInterval) {
      clearInterval(pollInterval);
    }
  });

  async function pollEvents() {
    try {
      const events = await invoke('poll_events');

      for (const event of events) {
        handleEvent(event);
      }
    } catch (e) {
      console.error('Poll error:', e);
    }
  }

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
        messages = newMessages;

        // Update users
        for (const profile of profiles) {
          users[profile.id] = profile;
        }
      }
    } else if (event.VkEvent) {
      handleVkEvent(event.VkEvent);
    } else if (event.Error) {
      status = `Ошибка: ${event.Error}`;
      console.error('Core error:', event.Error);
    }
  }

  function handleVkEvent(vkEvent) {
    if (vkEvent.NewMessage) {
      const { peer_id, text, from_id } = vkEvent.NewMessage;

      // Update unread counter for chat
      const chat = chats.find(c => c.id === peer_id);
      if (chat) {
        chat.unread_count = (chat.unread_count || 0) + 1;
        chat.last_message = text;
        chats = chats; // Trigger reactivity
      }

      // Add to messages if chat is selected
      if (selectedChat && selectedChat.id === peer_id) {
        messages = [...messages, {
          id: Date.now(),
          from_id,
          from_name: getUserName(from_id),
          text,
          timestamp: Math.floor(Date.now() / 1000),
          is_outgoing: false,
          is_read: true,
          is_edited: false,
          attachments: [],
        }];
      }
    } else if (vkEvent.ConnectionStatus !== undefined) {
      status = vkEvent.ConnectionStatus ? 'Подключено' : 'Отключено';
    } else if (vkEvent.UserTyping) {
      const { user_id } = vkEvent.UserTyping;
      status = `${getUserName(user_id)} печатает...`;

      setTimeout(() => {
        status = 'Готово';
      }, 3000);
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

    try {
      await invoke('load_messages', { peerId: chat.id, offset: 0 });
    } catch (e) {
      console.error('Failed to load messages:', e);
    }
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
</script>

<div class="main-view">
  <header class="header">
    <h2>Сообщения</h2>
    <div class="header-right">
      <span class="status">{status}</span>
      <button class="btn-logout" on:click={onLogout}>
        Выйти
      </button>
    </div>
  </header>

  <div class="content">
    <ChatList
      {chats}
      {loading}
      selectedChatId={selectedChat?.id}
      onSelectChat={handleChatSelect}
    />

    {#if selectedChat}
      <MessageView
        chat={selectedChat}
        {messages}
        {users}
        onSendMessage={handleSendMessage}
        onSendReply={handleSendReply}
      />
    {:else}
      <div class="empty-state">
        <p>Выберите чат</p>
      </div>
    {/if}
  </div>
</div>

<style>
  .main-view {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100vh;
    background: var(--cosmic-bg);
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem;
    background: var(--cosmic-surface-alt);
    border-bottom: 1px solid var(--cosmic-border);
  }

  h2 {
    font-size: 18px;
    font-weight: 600;
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .status {
    font-size: 12px;
    color: var(--cosmic-muted);
  }

  .btn-logout {
    padding: 0.5rem 1rem;
    background: var(--cosmic-surface);
    border: 1px solid var(--cosmic-border);
    border-radius: 8px;
    color: var(--cosmic-text);
    transition: background 0.2s;
  }

  .btn-logout:hover {
    background: #202638;
  }

  .content {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .empty-state {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--cosmic-muted);
  }
</style>

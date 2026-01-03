<script>
  import { onMount, tick } from 'svelte';
  import Message from './Message.svelte';
  import MessageInput from './MessageInput.svelte';

  export let chat;
  export let messages = [];
  export let users = {};
  export let onSendMessage;
  export let onSendReply;

  let messagesContainer;
  let replyTo = null;

  $: if (messages.length) {
    scrollToBottom();
  }

  async function scrollToBottom() {
    await tick();
    if (messagesContainer) {
      messagesContainer.scrollTop = messagesContainer.scrollHeight;
    }
  }

  function handleReply(message) {
    replyTo = message;
  }

  function handleCancelReply() {
    replyTo = null;
  }

  async function handleSend(text) {
    if (replyTo) {
      await onSendReply(replyTo.id, text);
      replyTo = null;
    } else {
      await onSendMessage(text);
    }
  }
</script>

<div class="message-view">
  <div class="messages-container" bind:this={messagesContainer}>
    {#if messages.length === 0}
      <div class="empty">
        <p>Нет сообщений</p>
      </div>
    {:else}
      {#each messages as message (message.id)}
        <Message {message} {users} onReply={handleReply} />
      {/each}
    {/if}
  </div>

  <MessageInput
    {replyTo}
    {users}
    onSend={handleSend}
    onCancelReply={handleCancelReply}
  />
</div>

<style>
  .message-view {
    flex: 1;
    display: flex;
    flex-direction: column;
    background: var(--cosmic-bg);
    overflow: hidden;
  }

  .messages-container {
    flex: 1;
    overflow-y: auto;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--cosmic-muted);
  }
</style>

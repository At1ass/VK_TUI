<script>
  import ForwardNode from './ForwardNode.svelte';

  export let message;
  export let users = {};
  export let onSelect;
  export let onContextMenu;
  export let isSelected = false;

  let forwardsOpen = false;

  function formatTime(timestamp) {
    const date = new Date(timestamp * 1000);
    const now = new Date();
    const diffHours = (now - date) / (1000 * 60 * 60);

    if (diffHours < 24) {
      // Today - show time only
      return date.toLocaleTimeString('ru-RU', {
        hour: '2-digit',
        minute: '2-digit',
      });
    } else if (diffHours < 168) {
      // This week - show day and time
      return date.toLocaleDateString('ru-RU', {
        weekday: 'short',
        hour: '2-digit',
        minute: '2-digit',
      });
    } else {
      // Older - show date
      return date.toLocaleDateString('ru-RU', {
        day: '2-digit',
        month: '2-digit',
        year: '2-digit',
      });
    }
  }

  function getUserName(userId) {
    const user = users[userId];
    if (user) {
      return `${user.first_name} ${user.last_name}`;
    }
    return `User ${userId}`;
  }

  function truncate(text, maxLen = 30) {
    if (text.length <= maxLen) return text;
    return text.substring(0, maxLen) + '...';
  }
</script>

<div
  class="message"
  class:outgoing={message.is_outgoing}
  class:selected={isSelected}
  data-message-id={message.id}
  on:click={(event) => onSelect?.(message, event)}
  on:contextmenu={(event) => {
    event.preventDefault();
    onContextMenu?.(message, event);
  }}
>
  <div class="message-bubble">
    <div class="message-header">
      <span class="sender">{message.from_name || getUserName(message.from_id)}</span>
      <span class="time">{formatTime(message.timestamp)}</span>
    </div>

    {#if message.reply}
      <div class="reply-preview">
        <div class="reply-bar"></div>
        <div class="reply-content">
          <p class="reply-author">{message.reply.from_name}</p>
          <p class="reply-text">{truncate(message.reply.text, 50)}</p>
        </div>
      </div>
    {/if}

    <p class="message-text document">{message.text}</p>

    {#if message.attachments && message.attachments.length > 0}
      <div class="attachments">
        {#each message.attachments as attachment}
          <div class="attachment">
            {#if attachment.type === 'photo'}
              <img src={attachment.url} alt="Attachment" class="attachment-image" />
            {:else if attachment.type === 'video'}
              <video src={attachment.url} controls class="attachment-video">
                <track kind="captions" />
              </video>
            {:else if attachment.type === 'audio'}
              <audio src={attachment.url} controls class="attachment-audio"></audio>
            {:else}
              <div class="attachment-doc">
                📎 {attachment.title || 'Файл'}
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {/if}

    {#if message.forwards && message.forwards.length > 0}
      <div class="forwards">
        <button class="button flat forward-toggle" on:click={() => (forwardsOpen = !forwardsOpen)}>
          {forwardsOpen ? 'Свернуть пересланные' : `Пересланные сообщения (${message.forwards.length})`}
        </button>
        {#if forwardsOpen}
          <div class="forwards-tree">
            {#each message.forwards as item}
              <ForwardNode item={item} level={0} defaultOpen={false} />
            {/each}
          </div>
        {/if}
      </div>
    {/if}

    <div class="message-footer">
      {#if message.is_edited}
        <span class="edited">изменено</span>
      {/if}
      {#if message.is_outgoing}
        <span class="read-status">
          {message.is_read ? '✓✓' : '✓'}
        </span>
      {/if}
    </div>
  </div>

</div>

<style>
.message {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  width: 100%;
  cursor: default;
  position: relative;
}

  .message-bubble {
    background: transparent;
    border-bottom: 1px solid var(--border-color);
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    border-left: 4px solid transparent;
    position: relative;
  }

  .message.selected .message-bubble {
    background: rgba(53, 132, 228, 0.32);
    border-left-color: var(--accent-bg-color);
    box-shadow: inset 0 0 0 1px rgba(53, 132, 228, 0.7);
  }

  .message.selected .message-bubble::before {
    content: '';
    position: absolute;
    inset: 0;
    border-left: 6px solid rgba(53, 132, 228, 0.95);
    pointer-events: none;
  }

  .message.outgoing .message-bubble {
    background: rgba(255, 255, 255, 0.03);
  }

  .message-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
  }

  .sender {
    font-weight: 600;
    font-size: 12px;
    color: var(--view-fg-color);
  }

  .message.outgoing .sender {
    color: var(--accent-bg-color);
  }

  .time {
    font-size: 10px;
    color: var(--muted-fg-color);
  }

  .reply-preview {
    display: flex;
    gap: 0.5rem;
    padding: 0.5rem;
    background: var(--accent-bg-color-dim);
    border-radius: var(--radius-s);
  }

  .reply-bar {
    width: 3px;
    background: var(--accent-bg-color);
    border-radius: 2px;
  }

  .reply-content {
    flex: 1;
  }

  .reply-author {
    font-size: 11px;
    font-weight: 600;
    color: var(--accent-bg-color);
    margin-bottom: 2px;
  }

  .reply-text {
    font-size: 11px;
    color: var(--muted-fg-color);
  }

  /* Document style class for message content - GNOME HIG */
  .message-text.document {
    font-family: var(--document-font-family);
    font-size: var(--document-font-size);
    line-height: 1.6;
    word-wrap: break-word;
    color: var(--view-fg-color);
  }

  .attachments {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .attachment-image {
    max-width: 100%;
    border-radius: var(--radius-m);
  }

  .attachment-video, .attachment-audio {
    max-width: 100%;
    border-radius: var(--radius-m);
  }

  .attachment-doc {
    padding: 0.5rem;
    background: var(--card-bg-color);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-s);
    font-size: 12px;
  }

  .forwards {
    font-size: 11px;
    color: var(--muted-fg-color);
    font-style: italic;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .forward-toggle {
    align-self: flex-start;
    font-size: 11px;
    color: var(--accent-bg-color);
  }

  .forwards-tree {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .message-footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    font-size: 10px;
    color: var(--muted-fg-color);
  }

</style>

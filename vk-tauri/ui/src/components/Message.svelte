<script>
  export let message;
  export let users = {};
  export let onReply;

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

<div class="message" class:outgoing={message.is_outgoing}>
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

    <p class="message-text">{message.text}</p>

    {#if message.attachments && message.attachments.length > 0}
      <div class="attachments">
        {#each message.attachments as attachment}
          <div class="attachment">
            {#if attachment.type === 'photo'}
              <img src={attachment.url} alt="Photo" class="attachment-image" />
            {:else if attachment.type === 'video'}
              <video src={attachment.url} controls class="attachment-video"></video>
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

    {#if message.fwd_count > 0}
      <div class="forwards">
        ↪ {message.fwd_count} переслано
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

  <button class="btn-reply" on:click={() => onReply(message)}>
    Ответить
  </button>
</div>

<style>
  .message {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    max-width: 70%;
    align-self: flex-start;
  }

  .message.outgoing {
    align-self: flex-end;
  }

  .message-bubble {
    background: var(--cosmic-surface);
    border: 1px solid var(--cosmic-border);
    border-radius: 12px;
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .message.outgoing .message-bubble {
    background: #14202e;
    border-color: #2a3243;
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
    color: var(--cosmic-accent);
  }

  .time {
    font-size: 10px;
    color: var(--cosmic-muted);
  }

  .reply-preview {
    display: flex;
    gap: 0.5rem;
    padding: 0.5rem;
    background: rgba(88, 170, 255, 0.1);
    border-radius: 6px;
  }

  .reply-bar {
    width: 3px;
    background: var(--cosmic-accent);
    border-radius: 2px;
  }

  .reply-content {
    flex: 1;
  }

  .reply-author {
    font-size: 11px;
    font-weight: 600;
    color: var(--cosmic-accent);
    margin-bottom: 2px;
  }

  .reply-text {
    font-size: 11px;
    color: var(--cosmic-muted);
  }

  .message-text {
    font-size: 14px;
    line-height: 1.4;
    word-wrap: break-word;
  }

  .attachments {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .attachment-image {
    max-width: 100%;
    border-radius: 8px;
  }

  .attachment-video, .attachment-audio {
    max-width: 100%;
    border-radius: 8px;
  }

  .attachment-doc {
    padding: 0.5rem;
    background: var(--cosmic-surface-alt);
    border-radius: 6px;
    font-size: 12px;
  }

  .forwards {
    font-size: 11px;
    color: var(--cosmic-muted);
    font-style: italic;
  }

  .message-footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    font-size: 10px;
    color: var(--cosmic-muted);
  }

  .btn-reply {
    align-self: flex-start;
    padding: 0.25rem 0.5rem;
    font-size: 11px;
    color: var(--cosmic-accent);
    opacity: 0;
    transition: opacity 0.2s;
  }

  .message:hover .btn-reply {
    opacity: 1;
  }

  .btn-reply:hover {
    text-decoration: underline;
  }
</style>

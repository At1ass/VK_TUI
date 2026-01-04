<script>
  export let replyTo = null;
  export let users = {};
  export let onSend;
  export let onCancelReply;

  let text = '';

  function handleKeydown(e) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  }

  function handleSubmit() {
    if (!text.trim()) return;

    onSend(text);
    text = '';
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

<div class="message-input-container">
  {#if replyTo}
    <div class="reply-indicator">
      <div class="reply-info">
        <span class="reply-label">Ответ на:</span>
        <span class="reply-author">{replyTo.from_name || getUserName(replyTo.from_id)}</span>
        <span class="reply-text">{truncate(replyTo.text, 40)}</span>
      </div>
      <button class="button flat" on:click={onCancelReply}>✕</button>
    </div>
  {/if}

  <div class="input-row">
    <textarea
      placeholder="Введите сообщение..."
      bind:value={text}
      on:keydown={handleKeydown}
      rows="1"
    ></textarea>

    <button class="button suggested" on:click={handleSubmit} disabled={!text.trim()}>
      Отправить
    </button>
  </div>
</div>

<style>
  .message-input-container {
    border-top: 1px solid var(--border-color);
    background: var(--headerbar-bg-color);
  }

  .reply-indicator {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.5rem 1rem;
    background: var(--accent-bg-color-dim);
    border-bottom: 1px solid var(--border-color);
  }

  .reply-indicator :global(.button) {
    min-height: 24px;
    padding: 0.15rem 0.35rem;
  }

  .reply-info {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 12px;
  }

  .reply-label {
    color: var(--muted-fg-color);
  }

  .reply-author {
    font-weight: 600;
    color: var(--accent-bg-color);
  }

  .reply-text {
    color: var(--muted-fg-color);
  }

  .input-row {
    display: flex;
    gap: 0.6rem;
    padding: 0.5rem 0.75rem;
  }

  textarea {
    flex: 1;
    padding: 0.5rem 0.6rem;
    background: var(--entry-bg-color);
    border: 1px solid var(--entry-border-color);
    border-radius: var(--radius-s);
    color: var(--view-fg-color);
    resize: none;
    min-height: 40px;
    max-height: 120px;
    font-family: inherit;
    font-size: 13px;
    line-height: 1.4;
    transition: border-color 0.2s;
  }

  textarea:focus {
    border-color: var(--accent-bg-color);
  }
</style>

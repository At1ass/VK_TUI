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
      <button class="btn-cancel" on:click={onCancelReply}>✕</button>
    </div>
  {/if}

  <div class="input-row">
    <textarea
      placeholder="Введите сообщение..."
      bind:value={text}
      on:keydown={handleKeydown}
      rows="1"
    ></textarea>

    <button class="btn-send" on:click={handleSubmit} disabled={!text.trim()}>
      Отправить
    </button>
  </div>
</div>

<style>
  .message-input-container {
    border-top: 1px solid var(--cosmic-border);
    background: var(--cosmic-surface);
  }

  .reply-indicator {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.5rem 1rem;
    background: rgba(88, 170, 255, 0.1);
    border-bottom: 1px solid var(--cosmic-border);
  }

  .reply-info {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 12px;
  }

  .reply-label {
    color: var(--cosmic-muted);
  }

  .reply-author {
    font-weight: 600;
    color: var(--cosmic-accent);
  }

  .reply-text {
    color: var(--cosmic-muted);
  }

  .btn-cancel {
    padding: 0.25rem 0.5rem;
    color: var(--cosmic-muted);
    font-size: 16px;
  }

  .btn-cancel:hover {
    color: var(--cosmic-text);
  }

  .input-row {
    display: flex;
    gap: 0.75rem;
    padding: 1rem;
  }

  textarea {
    flex: 1;
    padding: 0.75rem;
    background: var(--cosmic-surface-alt);
    border: 1px solid var(--cosmic-border);
    border-radius: 8px;
    color: var(--cosmic-text);
    resize: none;
    min-height: 40px;
    max-height: 120px;
    font-family: inherit;
    font-size: 14px;
    line-height: 1.4;
    transition: border-color 0.2s;
  }

  textarea:focus {
    border-color: var(--cosmic-accent);
  }

  .btn-send {
    padding: 0.75rem 1.5rem;
    background: var(--cosmic-accent);
    color: var(--cosmic-bg);
    border-radius: 8px;
    font-weight: 600;
    transition: all 0.2s;
  }

  .btn-send:hover:not(:disabled) {
    background: #6db9ff;
    box-shadow: 0 2px 8px rgba(88, 170, 255, 0.3);
  }

  .btn-send:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>

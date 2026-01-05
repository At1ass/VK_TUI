<script>
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';

  export let replyTo = null;
  export let users = {};
  export let peerId = null;
  export let onSend;
  export let onCancelReply;

  let text = '';
  let uploading = false;
  let isDragging = false;

  function handleKeydown(e) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  }

  function handleDragOver(e) {
    e.preventDefault();
    isDragging = true;
  }

  function handleDragLeave(e) {
    e.preventDefault();
    isDragging = false;
  }

  async function handleDrop(e) {
    e.preventDefault();
    isDragging = false;

    if (!peerId) return;

    const files = Array.from(e.dataTransfer.files);

    for (const file of files) {
      const path = file.path;
      if (!path) continue;

      const isImage = ['image/jpeg', 'image/png', 'image/gif', 'image/webp'].includes(file.type);

      try {
        uploading = true;
        if (isImage) {
          await invoke('send_photo', { peerId, path });
        } else {
          await invoke('send_doc', { peerId, path });
        }
      } catch (e) {
        console.error('Failed to send file:', e);
      } finally {
        uploading = false;
      }
    }
  }

  function handleSubmit() {
    if (!text.trim()) return;

    onSend(text);
    text = '';
  }

  async function handleAttachPhoto() {
    if (!peerId) return;

    try {
      const file = await open({
        multiple: false,
        filters: [{
          name: 'Images',
          extensions: ['jpg', 'jpeg', 'png', 'gif', 'webp']
        }]
      });

      if (file) {
        uploading = true;
        await invoke('send_photo', { peerId, path: file });
        uploading = false;
      }
    } catch (e) {
      console.error('Failed to attach photo:', e);
      uploading = false;
    }
  }

  async function handleAttachFile() {
    if (!peerId) return;

    try {
      const file = await open({
        multiple: false,
        filters: [{
          name: 'All Files',
          extensions: ['*']
        }]
      });

      if (file) {
        uploading = true;
        await invoke('send_doc', { peerId, path: file });
        uploading = false;
      }
    } catch (e) {
      console.error('Failed to attach file:', e);
      uploading = false;
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
  class="message-input-container"
  class:dragging={isDragging}
  on:dragover={handleDragOver}
  on:dragleave={handleDragLeave}
  on:drop={handleDrop}
>
  {#if isDragging}
    <div class="drag-overlay">
      <div class="drag-indicator">
        📎 Перетащите файл сюда
      </div>
    </div>
  {/if}

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
    <div class="attachment-buttons">
      <button
        class="button flat icon-button"
        on:click={handleAttachPhoto}
        disabled={!peerId || uploading}
        title="Прикрепить фото"
        aria-label="Прикрепить фото"
      >
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <rect x="3" y="3" width="18" height="18" rx="2" ry="2"/>
          <circle cx="8.5" cy="8.5" r="1.5"/>
          <polyline points="21 15 16 10 5 21"/>
        </svg>
      </button>

      <button
        class="button flat icon-button"
        on:click={handleAttachFile}
        disabled={!peerId || uploading}
        title="Прикрепить файл"
        aria-label="Прикрепить файл"
      >
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M21.44 11.05l-9.19 9.19a6 6 0 0 1-8.49-8.49l9.19-9.19a4 4 0 0 1 5.66 5.66l-9.2 9.19a2 2 0 0 1-2.83-2.83l8.49-8.48"/>
        </svg>
      </button>
    </div>

    <textarea
      placeholder="Введите сообщение..."
      bind:value={text}
      on:keydown={handleKeydown}
      rows="1"
    ></textarea>

    <button class="button suggested" on:click={handleSubmit} disabled={!text.trim() || uploading}>
      {uploading ? 'Отправка...' : 'Отправить'}
    </button>
  </div>
</div>

<style>
  .message-input-container {
    border-top: 1px solid var(--border-color);
    background: var(--headerbar-bg-color);
    position: relative;
  }

  .message-input-container.dragging {
    background: var(--accent-bg-color-dim);
  }

  .drag-overlay {
    position: absolute;
    inset: 0;
    z-index: 100;
    background: rgba(53, 132, 228, 0.15);
    backdrop-filter: blur(2px);
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
  }

  .drag-indicator {
    background: var(--accent-bg-color);
    color: white;
    padding: 1rem 2rem;
    border-radius: var(--radius-l);
    font-size: 14px;
    font-weight: 600;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
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
    align-items: flex-end;
  }

  .attachment-buttons {
    display: flex;
    gap: 0.25rem;
    align-items: center;
  }

  .attachment-buttons :global(.icon-button) {
    padding: 0.4rem;
    min-height: unset;
    color: var(--muted-fg-color);
  }

  .attachment-buttons :global(.icon-button:hover:not(:disabled)) {
    color: var(--accent-bg-color);
    background: var(--row-hover-bg-color);
  }

  .attachment-buttons :global(.icon-button:disabled) {
    opacity: 0.4;
    cursor: not-allowed;
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

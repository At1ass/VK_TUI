<script>
  import { onMount, tick } from 'svelte';
  import Message from './Message.svelte';
  import MessageInput from './MessageInput.svelte';

  export let chat = null;
  export let messages = [];
  export let users = {};
  export let chats = [];
  export let onSendMessage;
  export let onSendReply;
  export let onForward;
  export let onEditMessage;
  export let onDeleteMessages;
  export let onLoadMore;
  export let canLoadOlder = false;
  export let canLoadNewer = false;
  export let autoScroll = true;

  let messagesContainer;
  let replyTo = null;
  let currentChatId = null;
  let isPrepending = false;
  let pendingScrollTop = 0;
  let pendingScrollHeight = 0;
  let prevMessagesLength = 0;
  let isAtBottom = true;
  let pendingPrepend = false;
  let selectedIds = new Set();
  let lastSelectedId = null;
  let contextMenu = null;
  let contextMenuEl = null;
  let forwardModalOpen = false;
  let forwardMessageIds = [];
  let forwardComment = '';
  let forwardError = '';
  let forwardTargetId = null;
  let editModalOpen = false;
  let editTarget = null;
  let editText = '';
  let deleteModalOpen = false;
  let deleteMessageIds = [];
  let deleteError = '';

  $: if (messages.length && autoScroll && isAtBottom && !isPrepending) {
    scrollToBottom();
  }

  $: if (chat && chat.id !== currentChatId) {
    currentChatId = chat.id;
    replyTo = null;
    clearSelection();
  }

  async function scrollToBottom() {
    await tick();
    if (messagesContainer) {
      messagesContainer.scrollTop = messagesContainer.scrollHeight;
    }
  }

  async function adjustScrollAfterPrepend() {
    await tick();
    if (!messagesContainer) return;
    const newHeight = messagesContainer.scrollHeight;
    const delta = newHeight - pendingScrollHeight;
    messagesContainer.scrollTop = pendingScrollTop + delta;
    isPrepending = false;
    pendingPrepend = false;
  }

  function handleReply(message) {
    replyTo = message;
  }

  function handleCancelReply() {
    replyTo = null;
  }

  function handleScroll() {
    if (!messagesContainer) return;
    if (contextMenu) {
      closeContextMenu();
    }
    if (isPrepending || pendingPrepend) return;
    const top = messagesContainer.scrollTop;
    const height = messagesContainer.scrollHeight;
    const view = messagesContainer.clientHeight;
    const nearTop = top < 120;
    const nearBottom = height - (top + view) < 120;
    isAtBottom = nearBottom;

    if (nearTop && canLoadOlder && messages.length > 0) {
      const anchor = messages[0];
      if (anchor?.id) {
        isPrepending = true;
        pendingPrepend = true;
        pendingScrollTop = top;
        pendingScrollHeight = height;
        onLoadMore?.('older', anchor);
      }
    } else if (nearBottom && canLoadNewer && messages.length > 0) {
      const anchor = messages[messages.length - 1];
      if (anchor?.id) {
        onLoadMore?.('newer', anchor);
      }
    }
  }

  $: {
    const len = messages.length;
    if (isPrepending && len !== prevMessagesLength) {
      adjustScrollAfterPrepend();
    }
    prevMessagesLength = len;
  }

  async function handleSend(text) {
    if (replyTo) {
      await onSendReply(replyTo.id, text);
      replyTo = null;
    } else {
      await onSendMessage(text);
    }
  }

  function getMessageById(messageId) {
    return messages.find(m => m.id === messageId);
  }

  function selectMessage(message, event) {
    if (contextMenu) {
      closeContextMenu();
    }
    const multi = event.ctrlKey || event.metaKey || event.shiftKey;
    if (multi) {
      if (selectedIds.has(message.id)) {
        selectedIds.delete(message.id);
      } else {
        selectedIds.add(message.id);
      }
    } else {
      selectedIds = new Set([message.id]);
    }
    lastSelectedId = message.id;
    selectedIds = new Set(selectedIds);
  }

  async function openContextMenu(message, event) {
    if (!selectedIds.has(message.id)) {
      selectedIds = new Set([message.id]);
      lastSelectedId = message.id;
    }
    contextMenu = {
      x: event.clientX,
      y: event.clientY,
      message,
    };
    await tick();
    if (contextMenuEl) {
      const rect = contextMenuEl.getBoundingClientRect();
      const padding = 8;
      let x = event.clientX;
      let y = event.clientY;
      if (x + rect.width > window.innerWidth - padding) {
        x = window.innerWidth - rect.width - padding;
      }
      if (y + rect.height > window.innerHeight - padding) {
        y = window.innerHeight - rect.height - padding;
      }
      if (x < padding) x = padding;
      if (y < padding) y = padding;
      contextMenu = { ...contextMenu, x, y };
    }
  }

  function clearSelection() {
    selectedIds = new Set();
    lastSelectedId = null;
  }

  function closeContextMenu() {
    contextMenu = null;
  }

  function handleMenuReply() {
    const targetId = lastSelectedId ?? contextMenu?.message?.id;
    const target = targetId ? getMessageById(targetId) : null;
    if (!target) return;
    handleReply(target);
    clearSelection();
    closeContextMenu();
  }

  function handleMenuForward() {
    const ids = selectedIds.size ? Array.from(selectedIds) : [];
    if (!ids.length && contextMenu?.message?.id) {
      ids.push(contextMenu.message.id);
    }
    if (!ids.length) return;
    forwardMessageIds = ids;
    forwardComment = '';
    forwardError = '';
    forwardTargetId = chat?.id ?? null;
    forwardModalOpen = true;
    closeContextMenu();
  }

  function handleMenuEdit() {
    const ids = selectedIds.size ? Array.from(selectedIds) : [];
    const targetId = ids.length === 1 ? ids[0] : contextMenu?.message?.id;
    const target = targetId ? getMessageById(targetId) : null;
    if (!target || !target.is_outgoing) return;
    editTarget = target;
    editText = target.text || '';
    editModalOpen = true;
    closeContextMenu();
  }

  function handleMenuDelete() {
    const ids = selectedIds.size ? Array.from(selectedIds) : [];
    if (!ids.length && contextMenu?.message?.id) {
      ids.push(contextMenu.message.id);
    }
    if (!ids.length) return;
    deleteMessageIds = ids;
    deleteError = '';
    deleteModalOpen = true;
    closeContextMenu();
  }

  async function submitForward() {
    if (!forwardComment.trim()) {
      forwardError = 'Комментарий обязателен';
      return;
    }
    const targetId = Number(forwardTargetId);
    if (!Number.isFinite(targetId)) {
      forwardError = 'Выберите диалог';
      return;
    }
    await onForward(forwardMessageIds, forwardComment.trim(), targetId);
    forwardModalOpen = false;
    forwardMessageIds = [];
    forwardComment = '';
    forwardTargetId = null;
    clearSelection();
  }

  async function submitEdit() {
    if (!editTarget || !editText.trim()) {
      return;
    }
    await onEditMessage(editTarget.id, editTarget.cmid ?? null, editText.trim());
    editModalOpen = false;
    editTarget = null;
    editText = '';
    clearSelection();
  }

  async function submitDelete(forAll) {
    if (!deleteMessageIds.length) return;
    if (forAll) {
      const invalid = deleteMessageIds
        .map(getMessageById)
        .filter(m => m && !m.is_outgoing);
      if (invalid.length) {
        deleteError = 'Можно удалить для всех только свои сообщения';
        return;
      }
    }
    await onDeleteMessages(deleteMessageIds, forAll);
    deleteModalOpen = false;
    deleteMessageIds = [];
    deleteError = '';
    clearSelection();
  }

  function isMessageSelected(messageId) {
    return selectedIds.has(messageId);
  }

  onMount(() => {
    const closeOnClick = () => {
      if (contextMenu) {
        closeContextMenu();
      }
    };
    const closeOnEscape = (event) => {
      if (event.key === 'Escape') {
        forwardModalOpen = false;
        editModalOpen = false;
        deleteModalOpen = false;
        closeContextMenu();
        clearSelection();
      }
    };

    window.addEventListener('click', closeOnClick);
    window.addEventListener('keydown', closeOnEscape);

    return () => {
      window.removeEventListener('click', closeOnClick);
      window.removeEventListener('keydown', closeOnEscape);
    };
  });
</script>

  <div class="message-view">
    <div class="messages-container" bind:this={messagesContainer} on:scroll={handleScroll}>
    {#if messages.length === 0}
      <div class="empty">
        <p>Нет сообщений</p>
      </div>
    {:else}
      {#each messages as message (message.id)}
        <Message
          {message}
          {users}
          isSelected={isMessageSelected(message.id)}
          onSelect={selectMessage}
          onContextMenu={openContextMenu}
        />
      {/each}
    {/if}
  </div>

  {#if selectedIds.size > 0}
    <div class="selection-bar">
      <span>Выбрано: {selectedIds.size}</span>
      <button class="button flat" on:click={clearSelection}>Снять выделение</button>
    </div>
  {/if}

  <MessageInput
    {replyTo}
    {users}
    onSend={handleSend}
    onCancelReply={handleCancelReply}
  />

  {#if contextMenu}
    <button
      class="overlay"
      type="button"
      aria-label="Закрыть меню"
      on:click={closeContextMenu}
    ></button>
    <div
      class="context-menu"
      bind:this={contextMenuEl}
      style={`top: ${contextMenu.y}px; left: ${contextMenu.x}px;`}
      on:click|stopPropagation
    >
      <button on:click={handleMenuReply}>Ответить</button>
      <button on:click={handleMenuForward}>Переслать</button>
      <button on:click={handleMenuEdit}>Редактировать</button>
      <button on:click={handleMenuDelete}>Удалить</button>
    </div>
  {/if}

  {#if forwardModalOpen}
    <div class="modal-overlay">
      <div class="modal">
        <h3>Переслать сообщения</h3>
        {#if forwardError}
          <div class="error">{forwardError}</div>
        {/if}
        <label class="field">
          <span>Куда переслать</span>
          <select bind:value={forwardTargetId}>
            {#if chat}
              <option value={chat.id}>Текущий чат</option>
            {/if}
            {#each chats as item (item.id)}
              {#if !chat || item.id !== chat.id}
                <option value={item.id}>{item.title}</option>
              {/if}
            {/each}
          </select>
        </label>
        <textarea
          placeholder="Комментарий обязателен"
          bind:value={forwardComment}
          rows="4"
        ></textarea>
        <div class="modal-actions">
          <button class="button" on:click={() => (forwardModalOpen = false)}>Отмена</button>
          <button class="button suggested" on:click={submitForward}>Переслать</button>
        </div>
      </div>
    </div>
  {/if}

  {#if editModalOpen}
    <div class="modal-overlay">
      <div class="modal">
        <h3>Редактировать сообщение</h3>
        <textarea bind:value={editText} rows="4"></textarea>
        <div class="modal-actions">
          <button class="button" on:click={() => (editModalOpen = false)}>Отмена</button>
          <button class="button suggested" on:click={submitEdit}>Сохранить</button>
        </div>
      </div>
    </div>
  {/if}

  {#if deleteModalOpen}
    <div class="modal-overlay">
      <div class="modal">
        <h3>Удалить сообщения</h3>
        {#if deleteError}
          <div class="error">{deleteError}</div>
        {/if}
        <div class="modal-actions">
          <button class="button" on:click={() => submitDelete(false)}>Для себя</button>
          <button class="button destructive" on:click={() => submitDelete(true)}>Для всех</button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .message-view {
    flex: 1;
    display: flex;
    flex-direction: column;
    background: var(--view-bg-color);
    overflow: hidden;
  }

  .messages-container {
    flex: 1;
    overflow-y: auto;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  .selection-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.4rem 0.75rem;
    background: var(--card-bg-color);
    border-top: 1px solid var(--border-color);
    font-size: 12px;
  }

  .context-menu {
    position: fixed;
    z-index: 1000;
    background: var(--card-bg-color);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-l);
    padding: 0.25rem;
    display: flex;
    flex-direction: column;
    min-width: 160px;
    box-shadow: 0 8px 18px rgba(0, 0, 0, 0.3);
  }

  .context-menu button {
    text-align: left;
    padding: 0.5rem 0.75rem;
    border-radius: var(--radius-s);
    color: var(--view-fg-color);
    border: none;
    background: transparent;
  }

  .context-menu button:hover {
    background: var(--row-hover-bg-color);
  }

  .overlay {
    position: fixed;
    inset: 0;
    z-index: 900;
    background: transparent;
    border: none;
    padding: 0;
  }

  /* Dialog/Modal - GNOME HIG compliant */
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1100;
    backdrop-filter: blur(2px);
  }

  .modal {
    background: var(--dialog-bg-color);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 24px;
    min-width: 360px;
    max-width: 480px;
    width: min(480px, 90vw);
    display: flex;
    flex-direction: column;
    gap: 18px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
  }

  .modal h3 {
    font-size: 15px;
    font-weight: 700;
    color: var(--dialog-fg-color);
    margin: 0;
  }

  .modal textarea {
    width: 100%;
    background: var(--entry-bg-color);
    border: 1px solid var(--entry-border-color);
    border-radius: var(--radius-s);
    color: var(--view-fg-color);
    padding: 8px 12px;
    font-size: 13px;
    min-height: 80px;
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    margin-top: 6px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 12px;
    color: var(--muted-fg-color);
  }

  .field select {
    background: var(--entry-bg-color);
    border: 1px solid var(--entry-border-color);
    border-radius: var(--radius-s);
    color: var(--view-fg-color);
    padding: 6px 10px;
    font-size: 13px;
  }

  .error {
    font-size: 12px;
    color: var(--destructive-bg-color);
  }

  .empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--muted-fg-color);
  }
</style>

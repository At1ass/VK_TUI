<script>
  export let item;
  export let level = 0;
  export let onJumpToMessage = null;

  export let defaultOpen = false;
  let open = defaultOpen;

  function toggle() {
    if (item.nested && item.nested.length > 0) {
      open = !open;
    }
  }
</script>

<div class="forward-node" style={`padding-left: ${level * 12}px`}>
  <div class="forward-header">
    <button class="button flat toggle" on:click={toggle} disabled={!item.nested || item.nested.length === 0}>
      {#if item.nested && item.nested.length > 0}
        {open ? '▾' : '▸'}
      {:else}
        •
      {/if}
    </button>
    <span class="author">{item.from}</span>
    {#if onJumpToMessage && item.message_id}
      <button
        class="button flat jump-btn"
        on:click={(e) => { e.stopPropagation(); onJumpToMessage(item.message_id, item.peer_id); }}
        title="Перейти к сообщению"
      >
        ⤴
      </button>
    {/if}
    {#if item.nested && item.nested.length > 0}
      <button class="button flat toggle-label" on:click={toggle}>
        {open ? 'Свернуть' : 'Развернуть'}
      </button>
    {/if}
  </div>
  {#if item.text}
    <div class="forward-text">{item.text}</div>
  {/if}
  {#if item.attachments && item.attachments.length > 0}
    <div class="forward-attachments">
      {#each item.attachments as attachment}
        {#if (attachment.type === 'photo' || attachment.kind === 'Photo') && (attachment.thumbnail_url || attachment.url)}
          <img src={attachment.thumbnail_url || attachment.url} alt="Фото" class="forward-image" loading="lazy" decoding="async" />
        {:else if (attachment.type === 'sticker' || attachment.kind === 'Sticker') && attachment.url}
          <img src={attachment.url} alt="Стикер" class="forward-sticker" loading="lazy" decoding="async" />
        {:else}
          <span class="attachment-pill">
            {attachment.title || attachment.type || attachment.kind || 'Вложение'}
          </span>
        {/if}
      {/each}
    </div>
  {/if}
  {#if !open && item.nested && item.nested.length > 0}
    <button class="button flat forward-summary" on:click={toggle}>
      Показать ещё {item.nested.length} сообщений
    </button>
  {/if}
  {#if open && item.nested && item.nested.length > 0}
    <div class="forward-children">
      {#each item.nested as child}
        <svelte:self item={child} level={level + 1} defaultOpen={false} {onJumpToMessage} />
      {/each}
    </div>
    <button class="button flat forward-summary" on:click={toggle}>
      Свернуть
    </button>
  {/if}
</div>

<style>
  .forward-node {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: 0.35rem 0.5rem;
    border-left: 1px solid var(--border-color);
  }

  .forward-header {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 11px;
    color: var(--muted-fg-color);
  }

  .toggle {
    width: 18px;
    height: 18px;
    border-radius: var(--radius-s);
    color: var(--view-fg-color);
    font-size: 10px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }

  .toggle-label {
    font-size: 11px;
    color: var(--accent-bg-color);
    padding: 0;
  }

  .toggle:disabled {
    opacity: 0.5;
  }

  .author {
    font-weight: 600;
    flex: 1;
  }

  .jump-btn {
    font-size: 14px;
    color: var(--accent-bg-color);
    padding: 0 0.2rem;
    min-width: auto;
  }

  .jump-btn:hover {
    background: var(--row-hover-bg-color);
  }

  .forward-text {
    font-size: 12px;
    color: var(--view-fg-color);
  }

  .forward-summary {
    font-size: 11px;
    color: var(--muted-fg-color);
    align-self: flex-start;
    padding: 0;
    margin-top: 4px;
  }

  .forward-attachments {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
    align-items: center;
  }

  .attachment-pill {
    font-size: 10px;
    padding: 0.1rem 0.4rem;
    border-radius: 999px;
    background: var(--card-bg-color);
    border: 1px solid var(--border-color);
    color: var(--muted-fg-color);
  }

  .forward-image {
    width: 150px;
    height: 150px;
    border-radius: var(--radius-s);
    object-fit: cover;
    background: var(--card-bg-color);
  }

  .forward-sticker {
    width: 80px;
    height: 80px;
    object-fit: contain;
    background: transparent;
  }
</style>

<script>
  export let item;
  export let level = 0;

  let open = true;

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
  </div>
  {#if item.text}
    <div class="forward-text">{item.text}</div>
  {/if}
  {#if item.attachments && item.attachments.length > 0}
    <div class="forward-attachments">
      {#each item.attachments as attachment}
        <span class="attachment-pill">{attachment.type}</span>
      {/each}
    </div>
  {/if}
  {#if !open && item.nested && item.nested.length > 0}
    <div class="forward-summary">
      Ещё {item.nested.length} сообщений
    </div>
  {/if}
  {#if open && item.nested && item.nested.length > 0}
    <div class="forward-children">
      {#each item.nested as child}
        <svelte:self item={child} level={level + 1} />
      {/each}
    </div>
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

  .toggle:disabled {
    opacity: 0.5;
  }

  .author {
    font-weight: 600;
  }

  .forward-text {
    font-size: 12px;
    color: var(--view-fg-color);
  }

  .forward-summary {
    font-size: 11px;
    color: var(--muted-fg-color);
  }

  .forward-attachments {
    display: flex;
    gap: 0.25rem;
    flex-wrap: wrap;
  }

  .attachment-pill {
    font-size: 10px;
    padding: 0.1rem 0.4rem;
    border-radius: 999px;
    background: var(--card-bg-color);
    border: 1px solid var(--border-color);
    color: var(--muted-fg-color);
  }
</style>

<script lang="ts">
  /**
   * Folder Menu - picks the folder whose notes the carousel shows
   *
   * Folders are subdirectories of the cards directory (one level deep). The
   * menu also creates, renames and deletes folders, and moves the note being
   * edited to another folder.
   */

  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { cardStore, editingCard, NO_FOLDER, type FolderFilter } from "$lib/stores/cardStore";

  let open = false;
  let root: HTMLDivElement;
  let newName = "";
  /** Folder being renamed, and the name typed for it */
  let renaming: string | null = null;
  let renameTo = "";

  $: folders = $cardStore.folders;
  $: selected = $cardStore.folder;
  $: editing = $editingCard;
  $: counts = countByFolder($cardStore.cards);
  $: label = selected === null ? "All notes" : selected === NO_FOLDER ? "No folder" : selected;

  function countByFolder(cards: { folder: string }[]): Map<string, number> {
    const counts = new Map<string, number>();
    for (const card of cards) counts.set(card.folder, (counts.get(card.folder) ?? 0) + 1);
    return counts;
  }

  function toggle() {
    open = !open;
    renaming = null;
  }

  function choose(folder: FolderFilter) {
    cardStore.selectFolder(folder);
    open = false;
  }

  async function createFolder() {
    if (!newName.trim()) return;
    const name = await cardStore.createFolder(newName);
    if (name !== null) {
      newName = "";
      // A new note made right after this goes into the new folder
      cardStore.selectFolder(name);
    }
  }

  function startRename(folder: string) {
    renaming = folder;
    renameTo = folder;
  }

  async function finishRename() {
    if (renaming === null) return;
    const from = renaming;
    renaming = null;
    if (renameTo.trim() && renameTo.trim() !== from) {
      await cardStore.renameFolder(from, renameTo);
    }
  }

  async function deleteFolder(folder: string) {
    const count = counts.get(folder) ?? 0;
    const notes = count === 1 ? "1 note" : `${count} notes`;
    const question =
      count > 0
        ? `Delete folder “${folder}”? Its ${notes} will be kept outside folders.`
        : `Delete folder “${folder}”?`;
    if (confirm(question)) {
      await cardStore.deleteFolder(folder);
    }
  }

  async function moveEditing(event: Event) {
    if (!editing) return;
    await cardStore.moveCard(editing.id, (event.target as HTMLSelectElement).value);
  }

  async function openInFileManager() {
    open = false;
    try {
      await invoke("open_cards_directory");
    } catch (e) {
      console.error("Failed to open cards directory:", e);
    }
  }

  /** Escape closes the menu first (runs before the note editor's handler) */
  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape" && open) {
      event.preventDefault();
      if (renaming !== null) renaming = null;
      else open = false;
    }
  }

  onMount(() => {
    const closeOutside = (event: MouseEvent) => {
      if (open && !root.contains(event.target as Node)) open = false;
    };
    window.addEventListener("mousedown", closeOutside);
    window.addEventListener("keydown", handleKeydown, true);
    return () => {
      window.removeEventListener("mousedown", closeOutside);
      window.removeEventListener("keydown", handleKeydown, true);
    };
  });
</script>

<div class="folder-menu" bind:this={root} data-no-drag>
  <button
    class="folder-button"
    class:open
    on:click|stopPropagation={toggle}
    title="Folders"
    aria-haspopup="menu"
    aria-expanded={open}
  >
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
    </svg>
    <span class="folder-label">{label}</span>
    <svg class="chevron" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <path d="M6 9l6 6 6-6" />
    </svg>
  </button>

  {#if open}
    <div class="menu" role="menu">
      <button class="item" class:selected={selected === null} on:click={() => choose(null)}>
        <span class="name">All notes</span>
        <span class="count">{$cardStore.cards.length}</span>
      </button>
      <button class="item" class:selected={selected === NO_FOLDER} on:click={() => choose(NO_FOLDER)}>
        <span class="name muted">No folder</span>
        <span class="count">{counts.get(NO_FOLDER) ?? 0}</span>
      </button>

      {#if folders.length > 0}
        <div class="divider"></div>
        <div class="folder-list">
          {#each folders as folder (folder)}
            {#if renaming === folder}
              <form class="row-form" on:submit|preventDefault={finishRename}>
                <!-- svelte-ignore a11y-autofocus -->
                <input
                  bind:value={renameTo}
                  on:blur={finishRename}
                  aria-label="New name for {folder}"
                  autofocus
                />
              </form>
            {:else}
              <div class="item-row">
                <button class="item" class:selected={selected === folder} on:click={() => choose(folder)}>
                  <span class="name">{folder}</span>
                  <span class="count">{counts.get(folder) ?? 0}</span>
                </button>
                <button class="icon-button" on:click={() => startRename(folder)} title="Rename folder">
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M12 20h9" />
                    <path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z" />
                  </svg>
                </button>
                <button class="icon-button danger" on:click={() => deleteFolder(folder)} title="Delete folder">
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M3 6h18" />
                    <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
                  </svg>
                </button>
              </div>
            {/if}
          {/each}
        </div>
      {/if}

      <div class="divider"></div>
      <form class="row-form" on:submit|preventDefault={createFolder}>
        <input bind:value={newName} placeholder="New folder" aria-label="New folder name" />
        <button class="add-button" type="submit" disabled={!newName.trim()}>Add</button>
      </form>

      {#if editing}
        <div class="divider"></div>
        <label class="move">
          <span>Move this note to</span>
          <select value={editing.folder} on:change={moveEditing}>
            <option value={NO_FOLDER}>No folder</option>
            {#each folders as folder (folder)}
              <option value={folder}>{folder}</option>
            {/each}
          </select>
        </label>
      {/if}

      <div class="divider"></div>
      <button class="item" on:click={openInFileManager}>
        <span class="name muted">Open in file manager</span>
      </button>
    </div>
  {/if}
</div>

<style>
  .folder-menu {
    position: relative;
  }

  .folder-button {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    max-width: 170px;
    padding: 0.45rem 0.6rem;
    background: transparent;
    color: var(--text-secondary);
    border-radius: 8px;
    font-size: 0.875rem;
    cursor: pointer;
    transition:
      background var(--transition-fast),
      color var(--transition-fast);
  }

  .folder-button:hover,
  .folder-button.open {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .folder-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chevron {
    flex-shrink: 0;
  }

  .menu {
    position: absolute;
    top: calc(100% + 0.5rem);
    left: 0;
    width: 260px;
    padding: 0.4rem;
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 10px;
    box-shadow: var(--shadow-lg);
    cursor: default;
  }

  .folder-list {
    max-height: 300px;
    overflow-y: auto;
  }

  .item-row {
    display: flex;
    align-items: center;
  }

  .item {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    width: 100%;
    padding: 0.45rem 0.6rem;
    background: transparent;
    color: var(--text-primary);
    border-radius: 6px;
    font-size: 0.875rem;
    text-align: left;
    cursor: pointer;
  }

  .item:hover {
    background: var(--bg-hover);
  }

  .item.selected {
    background: var(--accent-glow);
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .muted {
    color: var(--text-secondary);
  }

  .count {
    flex-shrink: 0;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .icon-button {
    flex-shrink: 0;
    padding: 0.35rem;
    background: transparent;
    color: var(--text-muted);
    border-radius: 6px;
    cursor: pointer;
    opacity: 0;
    transition: opacity var(--transition-fast);
  }

  .item-row:hover .icon-button,
  .icon-button:focus-visible {
    opacity: 1;
  }

  .icon-button:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .icon-button.danger:hover {
    color: #ef4444;
  }

  .divider {
    height: 1px;
    margin: 0.35rem 0.25rem;
    background: var(--border-color);
  }

  .row-form {
    display: flex;
    gap: 0.4rem;
    padding: 0.2rem;
  }

  input,
  select {
    flex: 1;
    min-width: 0;
    padding: 0.4rem 0.55rem;
    background: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    font-size: 0.875rem;
  }

  input:focus,
  select:focus {
    outline: none;
    border-color: var(--accent-primary);
  }

  .add-button {
    padding: 0.4rem 0.75rem;
    background: var(--accent-primary);
    color: white;
    border-radius: 6px;
    font-size: 0.8125rem;
    font-weight: 500;
    cursor: pointer;
  }

  .add-button:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .move {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    padding: 0.3rem 0.4rem;
    font-size: 0.75rem;
    color: var(--text-secondary);
  }
</style>

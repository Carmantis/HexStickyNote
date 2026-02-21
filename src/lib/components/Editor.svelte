<script lang="ts">
  /**
   * Editor Component - CodeMirror 6 Markdown Editor
   *
   * Features:
   * - Markdown syntax highlighting
   * - Line numbers (optional)
   * - Dark theme (One Dark)
   * - Monospace font
   */

  import { onMount, onDestroy, createEventDispatcher } from 'svelte';
  import { EditorView, keymap, lineNumbers, highlightActiveLine } from '@codemirror/view';
  import { EditorState } from '@codemirror/state';
  import { markdown, markdownLanguage } from '@codemirror/lang-markdown';
  import { oneDark } from '@codemirror/theme-one-dark';
  import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
  import { syntaxHighlighting, defaultHighlightStyle } from '@codemirror/language';

  // Props
  export let content: string = '';
  export let showLineNumbers: boolean = true;
  export let autofocus: boolean = true;

  const dispatch = createEventDispatcher<{ change: string }>();

  let editorContainer: HTMLDivElement;
  let editorView: EditorView | null = null;

  // Create custom theme extensions
  const customTheme = EditorView.theme({
    '&': {
      fontSize: '14px',
      height: '100%'
    },
    '.cm-content': {
      fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
      padding: '0.5rem 0'
    },
    '.cm-gutters': {
      backgroundColor: 'var(--bg-secondary)',
      borderRight: '1px solid var(--border-color)'
    },
    '.cm-activeLineGutter': {
      backgroundColor: 'var(--bg-hover)'
    },
    '.cm-activeLine': {
      backgroundColor: 'rgba(99, 102, 241, 0.1)'
    },
    '&.cm-focused .cm-cursor': {
      borderLeftColor: 'var(--accent-primary)'
    },
    '&.cm-focused .cm-selectionBackground, ::selection': {
      backgroundColor: 'rgba(99, 102, 241, 0.3)'
    },
    '.cm-scroller': {
      overflow: 'auto'
    }
  });

  // Build extensions array
  function buildExtensions() {
    const extensions = [
      // Markdown language support
      markdown({ base: markdownLanguage }),

      // Syntax highlighting
      syntaxHighlighting(defaultHighlightStyle, { fallback: true }),

      // Dark theme
      oneDark,
      customTheme,

      // History (undo/redo)
      history(),

      // Keymaps
      keymap.of([...defaultKeymap, ...historyKeymap]),

      // Visual enhancements
      highlightActiveLine(),
      EditorView.lineWrapping,

      // Update listener
      EditorView.updateListener.of((update) => {
        if (update.docChanged) {
          const newContent = update.state.doc.toString();
          dispatch('change', newContent);
        }
      })
    ];

    // Optional line numbers
    if (showLineNumbers) {
      extensions.push(lineNumbers());
    }

    return extensions;
  }

  // Initialize editor
  function initEditor() {
    if (!editorContainer) return;

    // Clean up existing editor
    if (editorView) {
      editorView.destroy();
    }

    const state = EditorState.create({
      doc: content,
      extensions: buildExtensions()
    });

    editorView = new EditorView({
      state,
      parent: editorContainer
    });

    if (autofocus) {
      editorView.focus();
    }
  }

  // Update editor content externally
  export function setContent(newContent: string) {
    if (editorView && newContent !== editorView.state.doc.toString()) {
      editorView.dispatch({
        changes: {
          from: 0,
          to: editorView.state.doc.length,
          insert: newContent
        }
      });
    }
  }

  // Append content (for AI streaming)
  export function appendContent(text: string) {
    if (editorView) {
      const len = editorView.state.doc.length;
      editorView.dispatch({
        changes: { from: len, to: len, insert: text },
        selection: { anchor: len + text.length }
      });
    }
  }

  // Get current content
  export function getContent(): string {
    return editorView?.state.doc.toString() ?? content;
  }

  // Focus the editor
  export function focus() {
    editorView?.focus();
  }

  onMount(() => {
    initEditor();
  });

  onDestroy(() => {
    editorView?.destroy();
    editorView = null;
  });

  // Reinitialize if showLineNumbers changes
  $: if (editorContainer && editorView) {
    // Only reinit if the setting actually changed
    const _ = showLineNumbers;
  }

  // React to content prop changes (e.g. AI streaming)
  $: if (editorView && content !== undefined) {
    setContent(content);
  }
</script>

<div class="editor-wrapper" bind:this={editorContainer}></div>

<style>
  .editor-wrapper {
    flex: 1;
    min-height: 0; /* Allow shrinking */
    border-radius: 8px;
    overflow-y: auto; /* Scroll here */
    background: #1a1a24;
    border: 1px solid var(--border-color);
    display: flex;
    flex-direction: column;
    pointer-events: auto;
    user-select: text;
  }

  .editor-wrapper :global(.cm-editor) {
    flex: 1;
    height: auto;
    min-height: 100%;
  }

  .editor-wrapper :global(.cm-scroller) {
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    height: auto !important;
    overflow: visible;
  }
</style>

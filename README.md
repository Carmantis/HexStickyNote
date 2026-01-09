# HexStickyNote - Next-Gen AI Workspace

## Technical Specification v2.0

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        FRONTEND (Svelte)                        │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────┐    ┌──────────────┐    ┌─────────────────────┐    │
│  │   Orb   │───▶│     HUD      │───▶│   Card Carousel     │    │
│  │Launcher │    │  (Main View) │    │  (Rendered HTML)    │    │
│  └─────────┘    └──────────────┘    └──────────┬──────────┘    │
│                                                 │ click         │
│                                     ┌───────────▼───────────┐   │
│                                     │   Editor Modal        │   │
│                                     │  ┌─────────────────┐  │   │
│                                     │  │  CodeMirror 6   │  │   │
│                                     │  │  (Raw Markdown) │  │   │
│                                     │  └─────────────────┘  │   │
│                                     │  ┌─────────────────┐  │   │
│                                     │  │  AI Prompt Bar  │  │   │
│                                     │  └────────┬────────┘  │   │
│                                     └───────────┼───────────┘   │
├─────────────────────────────────────────────────┼───────────────┤
│                     TAURI IPC BRIDGE            │               │
├─────────────────────────────────────────────────┼───────────────┤
│                    BACKEND (Rust/Tauri v2)      ▼               │
│  ┌─────────────────┐  ┌─────────────────┐  ┌────────────────┐  │
│  │  KeyringStore   │  │   AiManager     │  │  CardStorage   │  │
│  │  (Credentials)  │◀─│  (API Router)   │  │  (SQLite/JSON) │  │
│  └────────┬────────┘  └────────┬────────┘  └────────────────┘  │
│           │                    │                                │
│           ▼                    ▼                                │
│  ┌─────────────────┐  ┌─────────────────────────────────────┐  │
│  │ Windows Cred.   │  │         External AI APIs            │  │
│  │    Locker       │  │  OpenAI │ Anthropic │ Google Gemini │  │
│  └─────────────────┘  └─────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 1. AI Architecture: BYOK (Bring Your Own Key)

### Supported Providers

| Provider  | Model              | Streaming | Endpoint                                      |
|-----------|--------------------|-----------|-----------------------------------------------|
| OpenAI    | gpt-4o             | Yes       | `https://api.openai.com/v1/chat/completions`  |
| Anthropic | claude-3.5-sonnet  | Yes       | `https://api.anthropic.com/v1/messages`       |
| Google    | gemini-1.5-pro     | Yes       | `https://generativelanguage.googleapis.com/v1beta/` |

### Security Model

```
┌──────────────────────────────────────────────────────────┐
│                    API Key Flow                          │
├──────────────────────────────────────────────────────────┤
│                                                          │
│  User Input ──▶ Rust Backend ──▶ Windows Credential     │
│                     │              Locker (Encrypted)    │
│                     │                    │               │
│                     ▼                    ▼               │
│              invoke_ai_stream() ◀── get_api_key()       │
│                     │                                    │
│                     ▼                                    │
│              HTTPS Request to AI Provider               │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

**Critical Security Rules:**
- API keys NEVER stored in plaintext files
- API keys NEVER exposed to frontend JavaScript
- All API calls made from Rust backend only
- Uses OS-level secure storage (Windows Credential Locker via `keyring` crate)

---

## 2. Card State Machine

```
                    ┌─────────────────┐
                    │   VIEW MODE     │
                    │  (Rendered HTML)│
                    │                 │
                    │  - markdown-it  │
                    │  - DOMPurify    │
                    │  - Read-only    │
                    └────────┬────────┘
                             │
                        click card
                             │
                             ▼
                    ┌─────────────────┐
                    │   EDIT MODE     │
                    │ (Raw Markdown)  │
                    │                 │
                    │  - CodeMirror 6 │
                    │  - Syntax HL    │
                    │  - AI Prompt    │
                    └────────┬────────┘
                             │
                      click outside /
                        save button
                             │
                             ▼
                    ┌─────────────────┐
                    │   VIEW MODE     │
                    │  (Re-rendered)  │
                    └─────────────────┘
```

---

## 3. Tech Stack

### Backend (Rust)
- **tauri** v2.x - Desktop framework
- **reqwest** - HTTP client with streaming
- **keyring** - OS credential storage
- **serde** / **serde_json** - Serialization
- **tokio** - Async runtime
- **futures** - Stream utilities

### Frontend (Svelte)
- **svelte** v4.x - UI framework
- **@codemirror/view** - Editor core
- **@codemirror/lang-markdown** - MD syntax
- **@codemirror/theme-one-dark** - Theme
- **marked** / **markdown-it** - MD parsing
- **dompurify** - HTML sanitization

---

## 4. File Structure

```
HexStickyNote/
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── ai_manager.rs
│       ├── keyring_store.rs
│       └── commands.rs
├── src/
│   ├── app.html
│   ├── app.css
│   ├── App.svelte
│   └── lib/
│       ├── stores/
│       │   ├── cardStore.ts
│       │   └── settingsStore.ts
│       └── components/
│           ├── Orb.svelte
│           ├── Hud.svelte
│           ├── Card.svelte
│           ├── CardCarousel.svelte
│           ├── Editor.svelte
│           ├── AiPromptBar.svelte
│           └── Settings.svelte
├── package.json
├── vite.config.ts
├── svelte.config.js
└── tsconfig.json
```

---

## 5. API Contracts

### Tauri Commands (IPC)

```typescript
// Settings
invoke('save_api_key', { provider: 'openai', key: 'sk-...' }): Promise<void>
invoke('get_providers'): Promise<Provider[]>
invoke('set_active_provider', { provider: 'openai' }): Promise<void>

// AI Streaming
invoke('invoke_ai_stream', { prompt: string, context: string }): Promise<void>
// Response via event: 'ai-stream-chunk' | 'ai-stream-end' | 'ai-stream-error'

// Cards
invoke('save_card', { card: Card }): Promise<void>
invoke('get_cards'): Promise<Card[]>
invoke('delete_card', { id: string }): Promise<void>
```

### Event Payloads

```typescript
interface AiStreamChunk {
  chunk: string;
  done: boolean;
}

interface AiStreamError {
  code: string;
  message: string;
}
```

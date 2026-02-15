# HexStickyNote

A modern desktop note-taking application with AI integration and a unique 3D carousel interface. Create, edit, and enhance your notes using powerful AI models – all while keeping your data and API keys secure on your local machine.

![HexStickyNote Screenshot](docs/image.png)



## Features

- **3D Carousel Interface**: Navigate your notes in an intuitive 3D carousel with smooth animations
- **AI-Powered Writing**: Enhance your notes with AI assistance from multiple providers:
  - OpenAI (GPT-4o, GPT-4o Mini, and more)
  - Anthropic (Claude 3.5 Sonnet, Claude 3.5 Haiku, and more)
  - Google (Gemini 2.5 Flash, Gemini 2.5 Pro, and more)
  - Local LLM (Llama-Poro-2 8B or any GGUF model) - works completely offline
- **Flexible Model Selection**: Choose from recommended models or specify your own custom model name
- **Markdown Support**: Write and render rich markdown content with syntax highlighting
- **Secure Credential Storage**: API keys are stored in Windows Credential Locker, never in plaintext files
- **Real-time Streaming**: AI responses stream in real-time for a responsive experience
- **Beautiful Glass Morphism UI**: Modern, transparent interface with blur effects
- **Portable Data**: Notes stored as human-readable markdown files with YAML front matter

## Installation

### For End Users

1. Download the latest installer from the [Releases](https://github.com/yourusername/HexStickyNote/releases) page
2. Run the `.msi` installer (Windows)
3. Launch HexStickyNote from the Start Menu

### For Developers

#### Prerequisites

- [Node.js](https://nodejs.org/) (v18 or later)
- [Rust](https://rustup.rs/) (latest stable)
- [LLVM](https://releases.llvm.org/) (for local LLM support)
  ```powershell
  # Windows (using Chocolatey)
  choco install llvm
  ```

#### Setup

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/HexStickyNote.git
   cd HexStickyNote
   ```

2. Install dependencies:
   ```bash
   npm install
   cd src-tauri
   cargo build
   cd ..
   ```

3. Set environment variable (for local LLM):
   ```powershell
   $env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"
   ```

4. Run in development mode:
   ```bash
   npm run tauri dev
   ```

## Building Installer Package

To create a distributable installer:

```bash
npm run tauri build
```

This will create:
- **Windows**: `.msi` installer in `src-tauri/target/release/bundle/msi/`
- The installer is code-signed if you have configured signing certificates in `tauri.conf.json`

### Build Options

- **Debug build** (faster, larger): `npm run tauri build -- --debug`
- **Release build** (optimized): `npm run tauri build` (default)

The installer will be located at:
```
src-tauri/target/release/bundle/msi/HexStickyNote_0.1.0_x64_en-US.msi
```

## Usage

### First Launch

1. Click the settings icon (gear) in the top-right corner
2. Choose an AI provider:
   - **Cloud providers**: Select provider and enter your API key
   - **Local Model**: Click "Download" to get the offline model (~4.4 GB)

### Creating Notes

1. Click the circular **+** button at the bottom center
2. A new blank card appears in the carousel

### Editing Notes

1. Click on any card to enter edit mode
2. Type your content in markdown format
3. Use the AI prompt bar at the bottom to enhance your note:
   - Type your prompt (e.g., "summarize this", "add examples")
   - Press Enter or click the send button
   - AI will stream the response directly into your note
4. Click outside the card or press ESC to save and exit

### Navigation

- **Arrow buttons**: Click left/right arrows to navigate
- **Keyboard**: Use `←` and `→` arrow keys
- **Indicators**: Click the dots at the bottom to jump to a specific card

### Window Controls

- **Move**: Click and drag the title bar
- **Settings**: Click the gear icon
- **Close**: Click the X button

## Data Storage

Your notes are stored as individual markdown files in:
```
%APPDATA%\HexStickyNote\HexStickyNote\cards\
```

Each note is saved as `{uuid}.md` with YAML front matter:
```markdown
---
id: "550e8400-e29b-41d4-a716-446655440000"
created_at: 1234567890
updated_at: 1234567890
---
# Your Note Title

Your markdown content here...
```

This format makes your notes:
- Human-readable
- Easy to edit with any text editor
- Version-controllable with Git
- Portable between systems
- Searchable with system tools

API keys are stored securely in Windows Credential Locker and never written to files.

## Configuration

### Local LLM Model

The local model is downloaded to:
```
%APPDATA%\HexStickyNote\models\
```

Current model: **Llama-Poro-2 8B Instruct** (GGUF Q4_K_M quantization)
- Size: ~4.4 GB
- Inference: CPU-only (no GPU required)

### GPU Acceleration (Optional)

For faster local inference, you can enable GPU support:

1. Edit `src-tauri/Cargo.toml` and uncomment GPU features:
   ```toml
   # For Vulkan (NVIDIA, AMD, Intel)
   llama-cpp-2 = { version = "0.1.132", features = ["vulkan"] }

   # OR for CUDA (NVIDIA only, fastest)
   llama-cpp-2 = { version = "0.1.132", features = ["cuda"] }
   ```

2. Install required SDK:
   - **Vulkan**: [Vulkan SDK](https://vulkan.lunarg.com/sdk/home)
   - **CUDA**: [CUDA Toolkit](https://developer.nvidia.com/cuda-downloads)

3. Rebuild the application:
   ```bash
   npm run tauri build
   ```

## Troubleshooting

### Build Errors

**Error 4551 (Application Control)**
- Windows Defender may block cargo build scripts
- Add exception in Windows Security → App & browser control

**"Cannot find LLVM"**
- Install LLVM and set `LIBCLANG_PATH` environment variable
- Restart your terminal after installation

**GPU features not working**
- Ensure SDK is installed (Vulkan SDK or CUDA Toolkit)
- Check environment variables (`VULKAN_SDK` or `PATH` includes `nvcc`)
- Restart terminal after setting environment variables

### Runtime Issues

**API calls failing**
- Verify your API key is correct
- Check your internet connection
- Ensure you have API credits with your provider

**Local model not working**
- Verify model file exists in `%APPDATA%\HexStickyNote\models\`
- Re-download the model from settings
- Check available RAM (model requires ~6 GB)

## Architecture

### Technology Stack

**Frontend:**
- Svelte 5 - Reactive UI framework
- CodeMirror 6 - Markdown editor with syntax highlighting
- marked - Markdown parser
- DOMPurify - HTML sanitization

**Backend:**
- Tauri v2 - Desktop framework (Rust + WebView)
- reqwest - HTTP client with streaming support
- keyring - Secure credential storage
- llama-cpp-2 - Local LLM inference
- tokio - Async runtime

### Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                   Frontend (Svelte)                     │
│  ┌──────────┐  ┌──────────┐  ┌────────────────────┐    │
│  │   HUD    │→ │ Carousel │→ │ Editor (CodeMirror)│    │
│  └──────────┘  └──────────┘  └─────────┬──────────┘    │
├─────────────────────────────────────────┼───────────────┤
│                 Tauri IPC Bridge        │               │
├─────────────────────────────────────────┼───────────────┤
│                Backend (Rust)           ▼               │
│  ┌────────────┐  ┌──────────┐  ┌──────────────────┐    │
│  │  Keyring   │◀─│AI Manager│  │  Card Storage    │    │
│  │  (Secure)  │  │(Streaming)  │  (Markdown)      │    │
│  └─────┬──────┘  └────┬─────┘  └──────────────────┘    │
│        │              │                                 │
│        ▼              ▼                                 │
│  Windows Cred.   Cloud APIs / Local LLM                 │
│     Locker       (OpenAI, Anthropic, Google, llama.cpp)│
└─────────────────────────────────────────────────────────┘
```

### Key Design Principles

1. **Security First**: API keys stored in OS-level secure storage, never in files or frontend
2. **Privacy**: Local LLM option for fully offline operation
3. **Streaming**: Real-time AI responses via Tauri event system
4. **Portability**: Notes stored as plain markdown files
5. **Modern UX**: 3D carousel with glass morphism design

### Card State Machine

```
┌─────────────┐
│  View Mode  │ (Rendered Markdown)
└──────┬──────┘
       │ click card
       ▼
┌─────────────┐
│  Edit Mode  │ (CodeMirror + AI Prompt)
└──────┬──────┘
       │ save / click outside
       ▼
┌─────────────┐
│  View Mode  │ (Re-rendered)
└─────────────┘
```

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.

### Development Workflow

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/amazing-feature`
3. Make your changes
4. Test thoroughly: `npm run tauri dev`
5. Commit with descriptive message: `git commit -m 'Add amazing feature'`
6. Push to your fork: `git push origin feature/amazing-feature`
7. Open a Pull Request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Built with [Tauri](https://tauri.app/) - Desktop app framework
- Icons from [Lucide](https://lucide.dev/)
- Local LLM powered by [llama.cpp](https://github.com/ggerganov/llama.cpp)
- Model: [Llama-Poro-2 8B Instruct](https://huggingface.co/LumiOpen/Llama-Poro-2-8B-Instruct) by LumiOpen

## Support

- **Issues**: [GitHub Issues](https://github.com/yourusername/HexStickyNote/issues)
- **Documentation**: See [CLAUDE.md](CLAUDE.md) for detailed technical documentation

---

Made with ❤️ using Tauri, Svelte, and Rust

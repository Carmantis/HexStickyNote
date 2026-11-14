# AI assistant

The assistant answers questions about your notes, calendar and tracked time, and makes changes to them when you approve. It runs on [Ollama](https://ollama.com) on your own computer; nothing is sent to a cloud service. To use Claude instead, connect [Claude Desktop](Claude-Desktop.md).

## Setting it up

1. Install Ollama and keep it running. HexStickyNote uses `OLLAMA_HOST` if it is set, like the Ollama command does.
2. Open **Settings → AI Model**.
3. Under **Add model from the Ollama library**, type a model name from [ollama.com/library](https://ollama.com/library), or a Hugging Face GGUF repository such as `hf.co/user/repo:Q4_K_M`, and click **Download**. The model is pulled into Ollama, with progress shown.
4. Pick it under **Model in use**.

The assistant needs a model that supports **tools**, such as `qwen3:8b` or `llama3.1:8b`. The list marks the models that do; with one that does not, the chat stays disabled and says why.

## Using it

The chat button in the top bar opens the assistant as a box at the bottom of the workspace; drag its top edge to make it taller or lower. It knows which view is open and which note is being edited, so "shorten this note" works on the note in front of you. **Save as note** under an answer keeps it as a new note, and **+** starts a new chat. The conversation is kept only while the app runs.

## What it can do

| Tool | Does | Needs approval |
| --- | --- | --- |
| `list_notes` | Lists notes with their title, folder and date | No |
| `read_note` | Reads a note | No |
| `create_note` | Creates a note, optionally in a folder | Yes |
| `update_note` | Replaces a note's content | Yes |
| `list_events` | Lists calendar events between two dates | No |
| `create_event` | Creates a calendar event | Yes |
| `list_time_entries` | Lists tracked time between two dates | No |
| `get_timer` | Shows the running timer | No |
| `start_timer` | Starts the timer, on an existing project if given | Yes |
| `stop_timer` | Stops the timer | Yes |

Reading happens straight away. A change is shown as a card describing exactly what will be done, with **Approve** and **Decline**; nothing changes before you approve, and a declined change is reported back to the model. The assistant cannot delete anything.

A model that keeps calling tools without answering is stopped after eight rounds.

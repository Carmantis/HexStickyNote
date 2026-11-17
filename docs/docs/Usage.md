# Using HexStickyNote

## The button and the workspace

HexStickyNote starts as a small round button that floats over other windows. Click it to open the workspace (the HUD) and again to close it; the app keeps running in the background. Drag the button with the right mouse button or with Ctrl + left button.

The workspace has three views, picked from the tabs in the top bar:

| View | What it is |
| --- | --- |
| **Notes** | Sticky notes in a 3D carousel, or as a grid of tiles |
| **Calendar** | HexCalendar: month, week and day views, day notes and reminders |
| **Time** | HexTime time tracking (see [Time tracking](Time-tracking.md)) |

The chat button and the gear on the right of the top bar open the [AI assistant](Assistant.md) and the settings, and × quits the app. The workspace is moved by dragging the top bar and resized from its edges. It remembers the last view.

## Notes

A note is Markdown. Its first `# Heading` is its title and its file name.

| To | Do |
| --- | --- |
| Create a note | **+** in the top bar. The note goes into the folder being viewed. |
| Open a note for editing | Click the card in front of the carousel |
| Save and close it | Esc, Ctrl+S, or click outside the card |
| Delete a note | Open it, then the bin in the top bar |
| Turn the carousel | ← and →, the arrow buttons, the dots, or click a card behind the front one |
| Reload notes changed outside the app | The refresh button next to the folder menu |

The carousel is a ring seen slightly from above. Only the card in front is sharp and can be opened; the others show blurred and see-through behind it.

### All notes at a glance

The four-square button next to the folder menu folds the carousel into a grid of small tiles, newest first, and back. Start typing to search the titles, text and folder names. Enter or a click on a tile brings that note to the front of the carousel and opens it; Esc clears the search, and a second Esc goes back to the carousel.

### Folders

The folder menu at the left of the top bar shows which folder is being viewed, with the number of notes in each:

- **All notes** shows every note; **No folder** the notes outside folders.
- **New folder** creates a folder and switches to it.
- The pencil and the bin on a folder's row rename and delete it. Deleting a folder keeps its notes: they move to **No folder**.
- While a note is open, **Move this note to** puts it in another folder.
- **Open in file manager** opens the notes directory.

Folders are directories next to the note files, one level deep, so they can also be made and filled in a file manager.

## Calendar

The calendar is HexCalendar built in, with the same database as the standalone HexCalendar. Events can have a reminder, which shows as a notification while HexStickyNote is running. In the event dialog, Ctrl+Enter saves and Esc closes.

## Time

The Time tab is HexTime: the timer, entries, projects, reports and PDF exports. HexStickyNote starts it the first time the tab is opened. PDFs are saved in the Downloads folder, with a notification. See [Time tracking](Time-tracking.md).

## Keys

| Key | Does |
| --- | --- |
| Ctrl+K | Open or close the workspace |
| Esc | Close the open menu, the search or the note being edited; otherwise close the workspace |
| Ctrl+S | Save and close the note being edited |
| ← / → | Turn the carousel |
| Enter (in the assistant) | Send; Shift+Enter starts a new line |

## Settings

| Section | Holds |
| --- | --- |
| **AI Model** | The Ollama model the assistant uses, and downloading new ones ([AI assistant](Assistant.md)) |
| **Claude Desktop** | Connecting Claude Desktop ([Claude Desktop](Claude-Desktop.md)) |

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";
import {
  createCard,
  listCards,
  readCard,
  updateCard,
  deleteCard,
} from "./cards.js";

const server = new McpServer({
  name: "hexstickynote",
  version: "0.1.0",
});

server.tool(
  "create_note",
  "Create a new sticky note with markdown content in HexStickyNote",
  {
    content: z.string().describe("The markdown content for the new note"),
  },
  async ({ content }) => {
    try {
      console.error(`[DEBUG] Creating card with content: ${content.substring(0, 50)}...`);
      const card = await createCard(content);
      console.error(`[DEBUG] Card created successfully: ${card.id}`);
      return {
        content: [{ type: "text" as const, text: JSON.stringify(card, null, 2) }],
      };
    } catch (err: any) {
      console.error(`[ERROR] Failed to create card: ${err.message}`);
      console.error(`[ERROR] Stack: ${err.stack}`);
      return {
        content: [{ type: "text" as const, text: `Error creating note: ${err.message}` }],
        isError: true,
      };
    }
  }
);

server.tool(
  "list_notes",
  "List all sticky notes in HexStickyNote with a preview of each note's content",
  {},
  async () => {
    try {
      const cards = await listCards();
      const previews = cards.map((card) => ({
        id: card.id,
        preview:
          card.content.slice(0, 200) +
          (card.content.length > 200 ? "..." : ""),
        created_at: new Date(card.created_at * 1000).toISOString(),
        updated_at: new Date(card.updated_at * 1000).toISOString(),
      }));
      return {
        content: [
          { type: "text" as const, text: JSON.stringify(previews, null, 2) },
        ],
      };
    } catch (err: any) {
      return {
        content: [{ type: "text" as const, text: `Error listing notes: ${err.message}` }],
        isError: true,
      };
    }
  }
);

server.tool(
  "read_note",
  "Read a specific sticky note by ID from HexStickyNote",
  {
    id: z.string().describe("The UUID of the note to read"),
  },
  async ({ id }) => {
    try {
      const card = await readCard(id);
      return {
        content: [{ type: "text" as const, text: JSON.stringify(card, null, 2) }],
      };
    } catch (err: any) {
      if ((err as NodeJS.ErrnoException).code === "ENOENT") {
        return {
          content: [{ type: "text" as const, text: `Note with ID ${id} not found.` }],
          isError: true,
        };
      }
      return {
        content: [{ type: "text" as const, text: `Error reading note: ${err.message}` }],
        isError: true,
      };
    }
  }
);

server.tool(
  "update_note",
  "Update an existing sticky note's content in HexStickyNote",
  {
    id: z.string().describe("The UUID of the note to update"),
    content: z.string().describe("The new markdown content for the note"),
  },
  async ({ id, content }) => {
    try {
      const card = await updateCard(id, content);
      return {
        content: [{ type: "text" as const, text: JSON.stringify(card, null, 2) }],
      };
    } catch (err: any) {
      if ((err as NodeJS.ErrnoException).code === "ENOENT") {
        return {
          content: [{ type: "text" as const, text: `Note with ID ${id} not found.` }],
          isError: true,
        };
      }
      return {
        content: [{ type: "text" as const, text: `Error updating note: ${err.message}` }],
        isError: true,
      };
    }
  }
);

server.tool(
  "delete_note",
  "Delete a sticky note from HexStickyNote",
  {
    id: z.string().describe("The UUID of the note to delete"),
  },
  async ({ id }) => {
    try {
      await deleteCard(id);
      return {
        content: [{ type: "text" as const, text: `Note ${id} deleted successfully.` }],
      };
    } catch (err: any) {
      if ((err as NodeJS.ErrnoException).code === "ENOENT") {
        return {
          content: [{ type: "text" as const, text: `Note with ID ${id} not found.` }],
          isError: true,
        };
      }
      return {
        content: [{ type: "text" as const, text: `Error deleting note: ${err.message}` }],
        isError: true,
      };
    }
  }
);

async function main() {
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error("HexStickyNote MCP server running on stdio");
}

main().catch((err) => {
  console.error("Fatal error:", err);
  process.exit(1);
});

//! AI Tools - Implements "MCP" pattern for AI interactions
//!
//! Provides tools that the LLM can call to interact with the application state.

use crate::card_manager;
use serde::{Deserialize, Serialize};
use serde_json::json;

// ============================================================================ 
// Tool Definitions
// ============================================================================ 

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String, // JSON string
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolResult {
    pub call_id: String,
    pub output: String,
}

/// Returns the JSON schema for all available tools
pub fn get_all_tools() -> serde_json::Value {
    json!([
        {
            "type": "function",
            "function": {
                "name": "create_note",
                "description": "Create a new sticky note card with the given content.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "string",
                            "description": "The markdown content of the new note."
                        }
                    },
                    "required": ["content"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "update_note",
                "description": "Update the content of an existing note card.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The UUID of the note to update."
                        },
                        "content": {
                            "type": "string",
                            "description": "The new markdown content."
                        }
                    },
                    "required": ["id", "content"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "delete_note",
                "description": "Delete a note card permanently.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The UUID of the note to delete."
                        }
                    },
                    "required": ["id"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_notes",
                "description": "Get a list of all existing notes (id, content, timestamps).",
                "parameters": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }
        }
    ])
}

// ============================================================================ 
// Tool Execution
// ============================================================================ 

#[derive(Deserialize)]
struct CreateNoteArgs {
    content: String,
}

#[derive(Deserialize)]
struct UpdateNoteArgs {
    id: String,
    content: String,
}

#[derive(Deserialize)]
struct DeleteNoteArgs {
    id: String,
}

/// Executes a tool call and returns the result as a string
pub fn execute_tool(name: &str, arguments: &str) -> Result<String, String> {
    match name {
        "create_note" => {
            let args: CreateNoteArgs = serde_json::from_str(arguments)
                .map_err(|e| format!("Invalid arguments for create_note: {}", e))?;
            
            let card = card_manager::create_card(args.content)
                .map_err(|e| format!("Failed to create card: {}", e))?;
            
            Ok(format!("Note created successfully. ID: {}", card.id))
        }
        "update_note" => {
            let args: UpdateNoteArgs = serde_json::from_str(arguments)
                .map_err(|e| format!("Invalid arguments for update_note: {}", e))?;
            
            card_manager::update_card(&args.id, Some(args.content))
                .map_err(|e| format!("Failed to update card: {}", e))?;
            
            Ok(format!("Note {} updated successfully.", args.id))
        }
        "delete_note" => {
            let args: DeleteNoteArgs = serde_json::from_str(arguments)
                .map_err(|e| format!("Invalid arguments for delete_note: {}", e))?;
            
            card_manager::delete_card(&args.id)
                .map_err(|e| format!("Failed to delete card: {}", e))?;
            
            Ok(format!("Note {} deleted successfully.", args.id))
        }
        "list_notes" => {
            let cards = card_manager::get_all_cards()
                .map_err(|e| format!("Failed to list cards: {}", e))?;
            
            // Format a concise list for the LLM
            let mut output = String::from("Current Notes:\n");
            if cards.is_empty() {
                output.push_str("(No notes found)");
            } else {
                for card in cards {
                    output.push_str(&format!("- ID: {}\n  Content (preview): {:.100}...\n", card.id, card.content.replace('\n', " ")));
                }
            }
            Ok(output)
        }
        _ => Err(format!("Unknown tool: {}", name)),
    }
}

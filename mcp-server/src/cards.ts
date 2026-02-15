import fs from "fs/promises";
import path from "path";
import yaml from "js-yaml";
import { v4 as uuidv4 } from "uuid";
import { Card, CardMetadata } from "./types.js";
import { getCardsDirectory } from "./paths.js";

// ============================================================================
// Helper Functions
// ============================================================================

// Extract title from markdown content (first # heading)
function extractTitleFromContent(content: string): string {
  for (const line of content.split('\n')) {
    const trimmed = line.trim();
    if (trimmed.startsWith('#')) {
      const title = trimmed.replace(/^#+\s*/, '').trim();
      if (title) {
        return title;
      }
    }
  }

  // Fallback: first non-empty line or "Untitled"
  const firstLine = content.split('\n').find(l => l.trim());
  return firstLine?.trim() || "Untitled";
}

// Sanitize title for use as filename
function sanitizeFilename(title: string): string {
  // Remove or replace invalid Windows filename characters: \ / : * ? " < > |
  let sanitized = title
    .replace(/\\/g, "-")
    .replace(/\//g, "-")
    .replace(/:/g, "-")
    .replace(/\*/g, "-")
    .replace(/\?/g, "")
    .replace(/"/g, "'")
    .replace(/</g, "(")
    .replace(/>/g, ")")
    .replace(/\|/g, "-");

  // Trim whitespace and dots from ends
  sanitized = sanitized.trim().replace(/\.+$/, '');

  // Limit length to 100 characters
  if (sanitized.length > 100) {
    sanitized = sanitized.substring(0, 100).trim();
  }

  // Ensure not empty
  if (!sanitized) {
    sanitized = "Untitled";
  }

  return sanitized;
}

// Get unique filename, handling duplicates
async function getUniqueFilename(cardsDir: string, baseName: string): Promise<string> {
  const filePath = path.join(cardsDir, `${baseName}.md`);
  try {
    await fs.access(filePath);
    // File exists, add number suffix
    let counter = 2;
    while (counter < 1000) {
      const numberedName = `${baseName} (${counter})`;
      const numberedPath = path.join(cardsDir, `${numberedName}.md`);
      try {
        await fs.access(numberedPath);
        counter++;
      } catch {
        return `${numberedName}.md`;
      }
    }
    // Safety limit reached, use UUID
    return `${uuidv4()}.md`;
  } catch {
    // File doesn't exist, use base name
    return `${baseName}.md`;
  }
}

// Find existing file by ID in front matter
async function findFileByID(cardsDir: string, id: string): Promise<string | null> {
  try {
    const entries = await fs.readdir(cardsDir);
    for (const entry of entries) {
      if (!entry.endsWith('.md')) continue;
      try {
        const filePath = path.join(cardsDir, entry);
        const content = await fs.readFile(filePath, 'utf-8');
        const { metadata } = parseMarkdownWithFrontmatter(content);
        if (metadata.id === id) {
          return filePath;
        }
      } catch {
        continue;
      }
    }
  } catch {
    // Ignore errors
  }
  return null;
}

function parseMarkdownWithFrontmatter(fileContent: string): {
  metadata: CardMetadata;
  content: string;
} {
  if (!fileContent.startsWith("---\n")) {
    throw new Error("File does not start with YAML front matter");
  }

  const rest = fileContent.slice(4);
  const endPos = rest.indexOf("\n---\n");

  if (endPos === -1) {
    throw new Error("Could not find closing --- for YAML front matter");
  }

  const yamlStr = rest.slice(0, endPos);
  const markdownContent = rest.slice(endPos + 5);

  const metadata = yaml.load(yamlStr) as CardMetadata;

  return { metadata, content: markdownContent };
}

function createMarkdownWithFrontmatter(card: Card): string {
  const metadata: CardMetadata = {
    id: card.id,
    created_at: card.created_at,
    updated_at: card.updated_at,
  };

  const yamlStr = yaml.dump(metadata, {
    lineWidth: -1,
    forceQuotes: false,
  });

  // Match Rust format: format!("---\n{}---\n{}", yaml, card.content)
  return `---\n${yamlStr}---\n${card.content}`;
}

async function ensureCardsDirectory(): Promise<string> {
  const dir = getCardsDirectory();
  await fs.mkdir(dir, { recursive: true });
  return dir;
}

// ============================================================================
// Public API
// ============================================================================

export async function createCard(content: string): Promise<Card> {
  const now = Math.floor(Date.now() / 1000);
  const card: Card = {
    id: uuidv4(),
    content,
    created_at: now,
    updated_at: now,
  };

  const dir = await ensureCardsDirectory();

  // Generate filename from content title
  const title = extractTitleFromContent(content);
  const sanitized = sanitizeFilename(title);
  const filename = await getUniqueFilename(dir, sanitized);
  const filePath = path.join(dir, filename);

  await fs.writeFile(filePath, createMarkdownWithFrontmatter(card), "utf-8");

  return card;
}

export async function listCards(): Promise<Card[]> {
  const dir = await ensureCardsDirectory();
  const entries = await fs.readdir(dir);
  const cards: Card[] = [];

  for (const entry of entries) {
    if (!entry.endsWith(".md")) continue;
    try {
      const filePath = path.join(dir, entry);
      const fileContent = await fs.readFile(filePath, "utf-8");
      const { metadata, content } = parseMarkdownWithFrontmatter(fileContent);
      cards.push({
        id: metadata.id,
        content,
        created_at: metadata.created_at,
        updated_at: metadata.updated_at,
      });
    } catch (err) {
      console.error(`Failed to load card from ${entry}: ${err}`);
    }
  }

  return cards;
}

export async function readCard(id: string): Promise<Card> {
  const dir = await ensureCardsDirectory();

  // Find file by ID in front matter
  const filePath = await findFileByID(dir, id);
  if (!filePath) {
    throw new Error(`Card with ID ${id} not found`);
  }

  const fileContent = await fs.readFile(filePath, "utf-8");
  const { metadata, content } = parseMarkdownWithFrontmatter(fileContent);
  return {
    id: metadata.id,
    content,
    created_at: metadata.created_at,
    updated_at: metadata.updated_at,
  };
}

export async function updateCard(id: string, content: string): Promise<Card> {
  const dir = await ensureCardsDirectory();

  // Find old file path
  const oldPath = await findFileByID(dir, id);
  if (!oldPath) {
    throw new Error(`Card with ID ${id} not found`);
  }

  const fileContent = await fs.readFile(oldPath, "utf-8");
  const { metadata } = parseMarkdownWithFrontmatter(fileContent);

  const updated: Card = {
    id: metadata.id,
    content,
    created_at: metadata.created_at,
    updated_at: Math.floor(Date.now() / 1000),
  };

  // Generate new filename from new content title
  const title = extractTitleFromContent(content);
  const sanitized = sanitizeFilename(title);
  const filename = await getUniqueFilename(dir, sanitized);
  const newPath = path.join(dir, filename);

  // Write to new file
  await fs.writeFile(newPath, createMarkdownWithFrontmatter(updated), "utf-8");

  // If filename changed, delete old file
  if (oldPath !== newPath) {
    try {
      await fs.unlink(oldPath);
    } catch {
      // Ignore error if old file doesn't exist
    }
  }

  return updated;
}

export async function deleteCard(id: string): Promise<void> {
  const dir = await ensureCardsDirectory();

  // Find file by ID
  const filePath = await findFileByID(dir, id);
  if (!filePath) {
    throw new Error(`Card with ID ${id} not found`);
  }

  await fs.unlink(filePath);
}

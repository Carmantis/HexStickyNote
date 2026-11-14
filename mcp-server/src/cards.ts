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

async function exists(filePath: string): Promise<boolean> {
  try {
    await fs.access(filePath);
    return true;
  } catch {
    return false;
  }
}

// `<base>.md` in `dir`, or `<base> (2).md` and so on when that is taken.
// `keep` is the note's current file: its own name does not count as taken.
async function getUniquePath(dir: string, baseName: string, keep?: string): Promise<string> {
  const isFree = async (candidate: string) => candidate === keep || !(await exists(candidate));

  const plain = path.join(dir, `${baseName}.md`);
  if (await isFree(plain)) return plain;
  for (let counter = 2; counter <= 1000; counter++) {
    const numbered = path.join(dir, `${baseName} (${counter}).md`);
    if (await isFree(numbered)) return numbered;
  }
  return path.join(dir, `${uuidv4()}.md`);
}

// A folder name as it is used on disk; "" stands for the cards directory itself
function cleanFolderName(name: string | undefined): string {
  const trimmed = (name ?? "").trim();
  if (!trimmed) return "";
  if (trimmed.startsWith(".")) {
    throw new Error(`"${trimmed}" is not a valid folder name`);
  }
  return sanitizeFilename(trimmed);
}

// Every note file with the folder (subdirectory, one level deep) it is in
async function noteFiles(cardsDir: string): Promise<{ folder: string; filePath: string }[]> {
  const files: { folder: string; filePath: string }[] = [];
  const entries = await fs.readdir(cardsDir, { withFileTypes: true });

  for (const entry of entries) {
    if (entry.isFile() && entry.name.endsWith(".md")) {
      files.push({ folder: "", filePath: path.join(cardsDir, entry.name) });
    } else if (entry.isDirectory() && !entry.name.startsWith(".")) {
      const folderDir = path.join(cardsDir, entry.name);
      for (const child of await fs.readdir(folderDir, { withFileTypes: true })) {
        if (child.isFile() && child.name.endsWith(".md")) {
          files.push({ folder: entry.name, filePath: path.join(folderDir, child.name) });
        }
      }
    }
  }
  return files;
}

// Find existing file by ID in front matter
async function findFileByID(
  cardsDir: string,
  id: string
): Promise<{ folder: string; filePath: string } | null> {
  try {
    for (const file of await noteFiles(cardsDir)) {
      try {
        const content = await fs.readFile(file.filePath, 'utf-8');
        const { metadata } = parseMarkdownWithFrontmatter(content);
        if (metadata.id === id) {
          return file;
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

export async function createCard(content: string, folder?: string): Promise<Card> {
  const now = Math.floor(Date.now() / 1000);
  const card: Card = {
    id: uuidv4(),
    content,
    created_at: now,
    updated_at: now,
    folder: cleanFolderName(folder),
  };

  const dir = path.join(await ensureCardsDirectory(), card.folder);
  await fs.mkdir(dir, { recursive: true });

  // Generate filename from content title
  const filePath = await getUniquePath(dir, sanitizeFilename(extractTitleFromContent(content)));
  await fs.writeFile(filePath, createMarkdownWithFrontmatter(card), "utf-8");

  return card;
}

export async function listCards(): Promise<Card[]> {
  const dir = await ensureCardsDirectory();
  const cards: Card[] = [];

  for (const { folder, filePath } of await noteFiles(dir)) {
    try {
      const fileContent = await fs.readFile(filePath, "utf-8");
      const { metadata, content } = parseMarkdownWithFrontmatter(fileContent);
      cards.push({
        id: metadata.id,
        content,
        created_at: metadata.created_at,
        updated_at: metadata.updated_at,
        folder,
      });
    } catch (err) {
      console.error(`Failed to load card from ${filePath}: ${err}`);
    }
  }

  return cards;
}

export async function readCard(id: string): Promise<Card> {
  const dir = await ensureCardsDirectory();

  // Find file by ID in front matter
  const file = await findFileByID(dir, id);
  if (!file) {
    throw new Error(`Card with ID ${id} not found`);
  }

  const fileContent = await fs.readFile(file.filePath, "utf-8");
  const { metadata, content } = parseMarkdownWithFrontmatter(fileContent);
  return {
    id: metadata.id,
    content,
    created_at: metadata.created_at,
    updated_at: metadata.updated_at,
    folder: file.folder,
  };
}

export async function updateCard(id: string, content: string): Promise<Card> {
  const dir = await ensureCardsDirectory();

  const file = await findFileByID(dir, id);
  if (!file) {
    throw new Error(`Card with ID ${id} not found`);
  }
  const oldPath = file.filePath;

  const fileContent = await fs.readFile(oldPath, "utf-8");
  const { metadata } = parseMarkdownWithFrontmatter(fileContent);

  const updated: Card = {
    id: metadata.id,
    content,
    created_at: metadata.created_at,
    updated_at: Math.floor(Date.now() / 1000),
    folder: file.folder,
  };

  // The note stays in its folder; the file is renamed when the title changed
  const newPath = await getUniquePath(
    path.dirname(oldPath),
    sanitizeFilename(extractTitleFromContent(content)),
    oldPath
  );

  await fs.writeFile(newPath, createMarkdownWithFrontmatter(updated), "utf-8");

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
  const file = await findFileByID(dir, id);
  if (!file) {
    throw new Error(`Card with ID ${id} not found`);
  }

  await fs.unlink(file.filePath);
}

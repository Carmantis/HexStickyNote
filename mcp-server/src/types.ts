export interface Card {
  id: string;
  content: string;
  created_at: number;
  updated_at: number;
  /** Subdirectory of the cards directory the note is in; "" for none */
  folder: string;
}

export interface CardMetadata {
  id: string;
  created_at: number;
  updated_at: number;
}

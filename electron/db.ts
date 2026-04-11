import fs from 'node:fs';
import path from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import type { SQLInputValue } from 'node:sqlite';
import { app } from 'electron';

import type { ClipFilter, ClipItem, ClipKind } from './types';

type ClipRow = {
  id: number;
  type: ClipKind;
  content_text: string;
  content_path: string | null;
  content_meta: string | null;
  is_favorite: number;
  created_at: string;
  updated_at: string;
};

type ClipMeta = {
  filePaths?: string[];
  imageWidth?: number;
  imageHeight?: number;
};

let database: DatabaseSync | null = null;

function getDatabase(): DatabaseSync {
  if (database) {
    return database;
  }

  const dataDir = path.join(app.getPath('userData'), 'data');
  fs.mkdirSync(dataDir, { recursive: true });
  const dbPath = path.join(dataDir, 'clipboard.db');

  database = new DatabaseSync(dbPath);
  database.exec('PRAGMA journal_mode = WAL;');
  database.exec(`
    CREATE TABLE IF NOT EXISTS clip_items (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      type TEXT NOT NULL,
      content_text TEXT NOT NULL,
      content_hash TEXT NOT NULL UNIQUE,
      content_path TEXT,
      content_meta TEXT,
      is_favorite INTEGER NOT NULL DEFAULT 0,
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL
    );

    CREATE INDEX IF NOT EXISTS idx_clip_items_updated_at
    ON clip_items(updated_at DESC);

    CREATE INDEX IF NOT EXISTS idx_clip_items_is_favorite
    ON clip_items(is_favorite, updated_at DESC);
  `);

  ensureColumn(database, 'content_path', 'TEXT');
  ensureColumn(database, 'content_meta', 'TEXT');

  return database;
}

function ensureColumn(db: DatabaseSync, columnName: string, columnDefinition: string): void {
  const columns = db.prepare('PRAGMA table_info(clip_items)').all() as Array<{ name: string }>;
  if (columns.some((column) => column.name === columnName)) {
    return;
  }

  db.exec(`ALTER TABLE clip_items ADD COLUMN ${columnName} ${columnDefinition};`);
}

function parseMeta(contentMeta: string | null): ClipMeta {
  if (!contentMeta) {
    return {};
  }

  try {
    return JSON.parse(contentMeta) as ClipMeta;
  } catch {
    return {};
  }
}

function toClipItem(row: ClipRow): ClipItem {
  const meta = parseMeta(row.content_meta);
  let previewDataUrl: string | null = null;

  if (row.type === 'image' && row.content_path && fs.existsSync(row.content_path)) {
    const imageBuffer = fs.readFileSync(row.content_path);
    previewDataUrl = `data:image/png;base64,${imageBuffer.toString('base64')}`;
  }

  return {
    id: row.id,
    type: row.type,
    contentText: row.content_text,
    isFavorite: row.is_favorite === 1,
    createdAt: row.created_at,
    updatedAt: row.updated_at,
    contentPath: row.content_path,
    filePaths: meta.filePaths ?? [],
    imageWidth: meta.imageWidth ?? null,
    imageHeight: meta.imageHeight ?? null,
    previewDataUrl,
  };
}

function upsertClip(
  type: ClipKind,
  contentText: string,
  contentHash: string,
  contentPath: string | null,
  contentMeta: ClipMeta,
): number | null {
  const normalized = contentText.trim();
  if (!normalized) {
    return null;
  }

  const db = getDatabase();
  const now = new Date().toISOString();
  const metaJson = JSON.stringify(contentMeta);
  const existing = db
    .prepare('SELECT id FROM clip_items WHERE content_hash = ?')
    .get(contentHash) as { id: number } | undefined;

  if (existing) {
    db.prepare(
      `UPDATE clip_items
       SET type = ?, content_text = ?, content_path = ?, content_meta = ?, updated_at = ?
       WHERE id = ?`,
    ).run(type, normalized, contentPath, metaJson, now, existing.id);
    return existing.id;
  }

  const result = db
    .prepare(
      `INSERT INTO clip_items (
        type, content_text, content_hash, content_path, content_meta, created_at, updated_at
      ) VALUES (?, ?, ?, ?, ?, ?, ?)`,
    )
    .run(type, normalized, contentHash, contentPath, metaJson, now, now);

  return Number(result.lastInsertRowid);
}

export function initDatabase(): void {
  getDatabase();
}

export function upsertTextClip(contentText: string, contentHash: string): number | null {
  return upsertClip('text', contentText, contentHash, null, {});
}

export function upsertImageClip(
  contentText: string,
  contentHash: string,
  contentPath: string,
  imageWidth: number,
  imageHeight: number,
): number | null {
  return upsertClip('image', contentText, contentHash, contentPath, {
    imageWidth,
    imageHeight,
  });
}

export function upsertFileClip(
  contentText: string,
  contentHash: string,
  filePaths: string[],
): number | null {
  return upsertClip('file', contentText, contentHash, null, {
    filePaths,
  });
}

export function listClips(search = '', filter: ClipFilter = 'all'): ClipItem[] {
  const db = getDatabase();
  const trimmed = search.trim();
  const conditions: string[] = [];
  const values: SQLInputValue[] = [];

  if (filter === 'favorite') {
    conditions.push('is_favorite = 1');
  } else if (filter !== 'all') {
    conditions.push('type = ?');
    values.push(filter);
  }

  if (trimmed) {
    conditions.push('content_text LIKE ?');
    values.push(`%${trimmed}%`);
  }

  const whereClause = conditions.length > 0 ? `WHERE ${conditions.join(' AND ')}` : '';
  const statement = db.prepare(
    `SELECT id, type, content_text, content_path, content_meta, is_favorite, created_at, updated_at
     FROM clip_items
     ${whereClause}
     ORDER BY updated_at DESC
     LIMIT 200`,
  );

  return (statement.all(...values) as ClipRow[]).map(toClipItem);
}

export function toggleFavorite(id: number): ClipItem | null {
  const db = getDatabase();
  db.prepare(
    `UPDATE clip_items
     SET is_favorite = CASE is_favorite WHEN 1 THEN 0 ELSE 1 END
     WHERE id = ?`,
  ).run(id);

  const row = db
    .prepare(
      `SELECT id, type, content_text, content_path, content_meta, is_favorite, created_at, updated_at
       FROM clip_items
       WHERE id = ?`,
    )
    .get(id) as ClipRow | undefined;

  return row ? toClipItem(row) : null;
}

export function getClipById(id: number): ClipItem | null {
  const row = getDatabase()
    .prepare(
      `SELECT id, type, content_text, content_path, content_meta, is_favorite, created_at, updated_at
       FROM clip_items
       WHERE id = ?`,
    )
    .get(id) as ClipRow | undefined;

  return row ? toClipItem(row) : null;
}

export function deleteClipsByIds(ids: number[]): number {
  if (ids.length === 0) {
    return 0;
  }

  const db = getDatabase();
  const placeholders = ids.map(() => '?').join(', ');
  const result = db
    .prepare(`DELETE FROM clip_items WHERE id IN (${placeholders})`)
    .run(...ids);

  return Number(result.changes ?? 0);
}

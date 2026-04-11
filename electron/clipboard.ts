import crypto from 'node:crypto';
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { app, clipboard } from 'electron';

import { upsertFileClip, upsertImageClip, upsertTextClip } from './db';
import type { ClipItem } from './types';

function getBlobDir(): string {
  const blobDir = path.join(app.getPath('userData'), 'blobs');
  fs.mkdirSync(blobDir, { recursive: true });
  return blobDir;
}

function toHash(value: string | Buffer): string {
  return crypto.createHash('sha256').update(value).digest('hex');
}

function parseFileUrls(text: string): string[] {
  return text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => {
      if (!line.startsWith('file://')) {
        return null;
      }

      try {
        return fileURLToPath(line);
      } catch {
        return null;
      }
    })
    .filter((value): value is string => value !== null);
}

function parseAbsoluteFilePaths(text: string): string[] {
  return text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .filter((line) => path.isAbsolute(line) && fs.existsSync(line));
}

function parsePlistFileEntries(text: string): string[] {
  const matches = Array.from(text.matchAll(/<string>(.*?)<\/string>/g))
    .map((match) => match[1]?.trim() ?? '')
    .filter(Boolean);

  const fileUrls = matches
    .filter((value) => value.startsWith('file://'))
    .map((value) => {
      try {
        return fileURLToPath(value);
      } catch {
        return null;
      }
    })
    .filter((value): value is string => value !== null);

  if (fileUrls.length > 0) {
    return fileUrls;
  }

  return matches.filter((value) => path.isAbsolute(value) && fs.existsSync(value));
}

function readClipboardFilePaths(): string[] {
  const nativePaths = readClipboardFilePathsViaSwift();
  if (nativePaths.length > 0) {
    return nativePaths;
  }

  const formats = clipboard.availableFormats();
  const candidateFormats = ['public.file-url', 'text/uri-list', 'NSFilenamesPboardType'];

  for (const format of candidateFormats) {
    if (!formats.includes(format)) {
      continue;
    }

    const buffer = clipboard.readBuffer(format);
    const rawText = buffer.toString('utf8').replace(/\0/g, '');
    const rawUrls = parseFileUrls(rawText);
    if (rawUrls.length > 0) {
      return rawUrls;
    }

    const rawPlistEntries = parsePlistFileEntries(rawText);
    if (rawPlistEntries.length > 0) {
      return rawPlistEntries;
    }

    const rawPaths = parseAbsoluteFilePaths(rawText);
    if (rawPaths.length > 0) {
      return rawPaths;
    }
  }

  const clipboardText = clipboard.readText();
  const parsedUrls = parseFileUrls(clipboardText);
  if (parsedUrls.length > 0) {
    return parsedUrls;
  }

  return parseAbsoluteFilePaths(clipboardText);
}

function readClipboardFilePathsViaSwift(): string[] {
  try {
    const output = execFileSync(
      'osascript',
      [
        '-e',
        'try',
        '-e',
        'set fileRef to (the clipboard as «class furl») as text',
        '-e',
        'return POSIX path of (fileRef as alias)',
        '-e',
        'on error',
        '-e',
        'return ""',
        '-e',
        'end try',
      ],
      {
        encoding: 'utf8',
      },
    );

    return parseAbsoluteFilePaths(output.trim());
  } catch {
    return [];
  }
}

export class ClipboardWatcher {
  private readonly intervalMs: number;
  private timer: NodeJS.Timeout | null = null;
  private lastHash = '';
  private suppressedHashes = new Set<string>();
  private readonly onChange: () => void;

  constructor(onChange: () => void, intervalMs = 500) {
    this.intervalMs = intervalMs;
    this.onChange = onChange;
  }

  start(): void {
    if (this.timer) {
      return;
    }

    this.captureCurrentClipboard();
    this.timer = setInterval(() => {
      this.captureCurrentClipboard();
    }, this.intervalMs);
  }

  stop(): void {
    if (!this.timer) {
      return;
    }

    clearInterval(this.timer);
    this.timer = null;
  }

  suppressClip(clip: ClipItem): void {
    if (clip.type === 'image') {
      if (!clip.contentPath || !fs.existsSync(clip.contentPath)) {
        return;
      }

      this.suppressedHashes.add(toHash(fs.readFileSync(clip.contentPath)));
      return;
    }

    if (clip.type === 'file') {
      this.suppressedHashes.add(toHash(clip.filePaths.join('\n')));
      return;
    }

    this.suppressedHashes.add(toHash(clip.contentText));
  }

  private shouldSuppress(contentHash: string): boolean {
    if (!this.suppressedHashes.has(contentHash)) {
      return false;
    }

    this.suppressedHashes.delete(contentHash);
    this.lastHash = contentHash;
    return true;
  }

  private captureCurrentClipboard(): void {
    const filePaths = readClipboardFilePaths();
    if (filePaths.length > 0) {
      const summary = filePaths.join('\n');
      const contentHash = toHash(summary);
      if (this.shouldSuppress(contentHash)) {
        return;
      }
      if (contentHash === this.lastHash) {
        return;
      }

      this.lastHash = contentHash;
      const insertedId = upsertFileClip(summary, contentHash, filePaths);
      if (insertedId !== null) {
        this.onChange();
      }
      return;
    }

    const image = clipboard.readImage();
    if (!image.isEmpty()) {
      const pngBuffer = image.toPNG();
      const contentHash = toHash(pngBuffer);
      if (this.shouldSuppress(contentHash)) {
        return;
      }
      if (contentHash === this.lastHash) {
        return;
      }

      this.lastHash = contentHash;
      const blobPath = path.join(getBlobDir(), `${contentHash}.png`);
      if (!fs.existsSync(blobPath)) {
        fs.writeFileSync(blobPath, pngBuffer);
      }

      const size = image.getSize();
      const summary = `${size.width} x ${size.height}`;
      const insertedId = upsertImageClip(summary, contentHash, blobPath, size.width, size.height);
      if (insertedId !== null) {
        this.onChange();
      }
      return;
    }

    const text = clipboard.readText();
    if (!text.trim()) {
      return;
    }

    const contentHash = toHash(text);
    if (this.shouldSuppress(contentHash)) {
      return;
    }
    if (contentHash === this.lastHash) {
      return;
    }

    this.lastHash = contentHash;
    const insertedId = upsertTextClip(text, contentHash);
    if (insertedId !== null) {
      this.onChange();
    }
  }
}

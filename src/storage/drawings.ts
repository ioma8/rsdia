// Drawing files: ${XDG_DATA_HOME:-~/.local/share}/lazydraw/drawings/<slug>.ld.json
import { mkdirSync, readdirSync, readFileSync, renameSync, rmSync, statSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { basename, dirname, join } from "node:path";
import { Layer } from "../core/layer";
import { Vector } from "../core/vector";

export const FILE_EXT = ".ld.json";

export function dataDir(): string {
  return join(process.env.XDG_DATA_HOME || join(homedir(), ".local", "share"), "lazydraw");
}

export function drawingsDir(): string {
  return join(dataDir(), "drawings");
}

export interface DrawingFile {
  version: 1;
  name: string;
  createdAt: string;
  updatedAt: string;
  cells: [number, number, string][];
}

export interface DrawingInfo {
  name: string;
  path: string;
  /** Number of non-empty cells. */
  size: number;
  updatedAt: string;
}

export function slugify(name: string): string {
  const slug = name
    .normalize("NFKD")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return slug || "untitled";
}

export function pathForName(name: string, dir = drawingsDir()): string {
  return join(dir, `${slugify(name)}${FILE_EXT}`);
}

/** Cells sorted row-major so saves are deterministic. */
export function serialize(name: string, layer: Layer, createdAt: string, updatedAt: string): string {
  const cells = layer
    .entries()
    .filter(([, v]) => v !== "" && v !== " ")
    .sort(([a], [b]) => a.y - b.y || a.x - b.x)
    .map(([p, v]) => [p.x, p.y, v] as [number, number, string]);
  const lines = cells.map((c) => `    ${JSON.stringify(c)}`);
  return [
    "{",
    `  "version": 1,`,
    `  "name": ${JSON.stringify(name)},`,
    `  "createdAt": ${JSON.stringify(createdAt)},`,
    `  "updatedAt": ${JSON.stringify(updatedAt)},`,
    cells.length ? `  "cells": [\n${lines.join(",\n")}\n  ]` : `  "cells": []`,
    "}",
    "",
  ].join("\n");
}

export function deserialize(text: string): { file: DrawingFile; layer: Layer } {
  const file = JSON.parse(text) as DrawingFile;
  if (file.version !== 1 || !Array.isArray(file.cells)) throw new Error("Unsupported drawing file");
  const layer = new Layer();
  for (const [x, y, v] of file.cells) layer.set(new Vector(x, y), v);
  return { file, layer };
}

export class DrawingStore {
  constructor(readonly dir = drawingsDir()) {}

  private ensureDir() {
    mkdirSync(this.dir, { recursive: true });
  }

  list(): DrawingInfo[] {
    let names: string[];
    try {
      names = readdirSync(this.dir).filter((n) => n.endsWith(FILE_EXT));
    } catch {
      return [];
    }
    const out: DrawingInfo[] = [];
    for (const n of names) {
      const path = join(this.dir, n);
      try {
        const { file } = deserialize(readFileSync(path, "utf8"));
        out.push({ name: file.name, path, size: file.cells.length, updatedAt: file.updatedAt });
      } catch {
        // Skip unreadable files rather than failing the whole list.
      }
    }
    return out.sort((a, b) => a.name.localeCompare(b.name));
  }

  exists(name: string): boolean {
    const slug = slugify(name);
    return this.list().some((d) => slugify(d.name) === slug);
  }

  /** "untitled", "untitled 2", ... — first name not taken. */
  uniqueName(base = "untitled"): string {
    if (!this.exists(base)) return base;
    for (let i = 2; ; i++) if (!this.exists(`${base} ${i}`)) return `${base} ${i}`;
  }

  pathFor(name: string): string {
    return pathForName(name, this.dir);
  }

  loadSync(path: string): { file: DrawingFile; layer: Layer } {
    return deserialize(readFileSync(path, "utf8"));
  }

  /** Atomic write: temp file + rename. */
  save(path: string, name: string, layer: Layer, createdAt: string): string {
    const updatedAt = new Date().toISOString();
    mkdirSync(dirname(path), { recursive: true });
    const tmp = join(dirname(path), `.${basename(path)}.tmp`);
    writeFileSync(tmp, serialize(name, layer, createdAt, updatedAt));
    renameSync(tmp, path);
    return updatedAt;
  }

  rename(oldPath: string, newName: string, layer: Layer, createdAt: string): string {
    const newPath = this.pathFor(newName);
    this.save(newPath, newName, layer, createdAt);
    if (newPath !== oldPath) rmSync(oldPath, { force: true });
    return newPath;
  }

  delete(path: string): void {
    rmSync(path, { force: true });
  }

  modifiedAt(path: string): number {
    try {
      return statSync(path).mtimeMs;
    } catch {
      return 0;
    }
  }

  create(name: string): string {
    this.ensureDir();
    const path = this.pathFor(name);
    this.save(path, name, new Layer(), new Date().toISOString());
    return path;
  }
}

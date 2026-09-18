import { afterAll, describe, expect, test } from "bun:test";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { exportText } from "../../src/core/export";
import { textToLayer } from "../../src/core/text";
import { loadConfig, saveConfig } from "../../src/storage/config";
import { deserialize, DrawingStore, serialize, slugify } from "../../src/storage/drawings";

const dir = mkdtempSync(join(tmpdir(), "lazydraw-storage-"));
afterAll(() => rmSync(dir, { recursive: true, force: true }));

const CLI = join(import.meta.dir, "../../src/cli.ts");

describe("drawings", () => {
  test("save/load round-trip is byte-identical (M8)", () => {
    const layer = textToLayer("┌──┐\n│ \"x\" │\n└──┘ ☃");
    const text = serialize("round trip", layer, "2026-01-01T00:00:00.000Z", "2026-01-02T00:00:00.000Z");
    const { file, layer: loaded } = deserialize(text);
    expect(serialize(file.name, loaded, file.createdAt, file.updatedAt)).toBe(text);
  });

  test("store: create, list, rename, delete, unique names", () => {
    const store = new DrawingStore(join(dir, "store"));
    expect(store.list()).toEqual([]);
    const a = store.create("untitled");
    expect(store.uniqueName()).toBe("untitled 2");
    store.save(a, "untitled", textToLayer("hello"), "2026-01-01T00:00:00.000Z");
    expect(store.list()).toMatchObject([{ name: "untitled", size: 5 }]);
    const b = store.rename(a, "Big Diagram!", store.loadSync(a).layer, "2026-01-01T00:00:00.000Z");
    expect(b.endsWith("big-diagram.ld.json")).toBe(true);
    expect(store.exists("untitled")).toBe(false);
    expect(store.exists("big diagram")).toBe(true);
    store.delete(b);
    expect(store.list()).toEqual([]);
  });

  test("slugify", () => {
    expect(slugify("  Café Plan / v2 ")).toBe("cafe-plan-v2");
    expect(slugify("***")).toBe("untitled");
  });
});

describe("config", () => {
  test("round-trips and sanitises", () => {
    const path = join(dir, "config.json");
    expect(loadConfig(path).grid).toBe("lattice");
    const c = loadConfig(path);
    c.grid = "checker";
    c.export.wrapper = "hash";
    saveConfig(c, path);
    expect(loadConfig(path)).toEqual(c);
    Bun.write(path, JSON.stringify({ grid: "bogus", theme: "gruvbox" }));
    const bad = loadConfig(path);
    expect(bad.grid).toBe("lattice");
    expect(bad.theme).toBe("gruvbox");
  });
});

describe("cli", () => {
  test("export command", () => {
    const store = new DrawingStore(join(dir, "cli", "lazydraw", "drawings"));
    const layer = textToLayer("┌┐\n└┘");
    store.save(store.pathFor("cli test"), "cli test", layer, "2026-01-01T00:00:00.000Z");
    const env = { ...process.env, XDG_DATA_HOME: join(dir, "cli"), XDG_CONFIG_HOME: join(dir, "cli-config") };
    const run = (...args: string[]) => Bun.spawnSync(["bun", CLI, ...args], { env });

    const plain = run("export", "cli test");
    expect(plain.exitCode).toBe(0);
    expect(plain.stdout.toString()).toBe("┌┐\n└┘\n");

    const out = join(dir, "out.txt");
    expect(run("export", "cli test", "--basic", "--comment", "hashes", "--fenced", "-o", out).exitCode).toBe(0);
    expect(readFileSync(out, "utf8")).toBe(
      `${exportText(layer, { characters: "basic", wrapper: "hash", fenced: true })}\n`,
    );

    const missing = run("export", "nope");
    expect(missing.exitCode).toBe(1);
    expect(missing.stderr.toString()).toContain('no drawing named "nope"');

    expect(run("--version").stdout.toString().trim()).toMatch(/^\d+\.\d+\.\d+$/);
  });
});

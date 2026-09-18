// End-to-end: the real App on OpenTUI's headless test renderer.
import { afterEach, describe, expect, test } from "bun:test";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createTestRenderer, type TestRendererSetup } from "@opentui/core/testing";
import { Layer } from "../../src/core/layer";
import { DEFAULT_CONFIG } from "../../src/storage/config";
import { DrawingStore } from "../../src/storage/drawings";
import { App } from "../../src/tui/app";
import { BAR_Y } from "../../src/tui/toolbar";
import { palette, type TerminalColors } from "../../src/tui/theme";

interface Harness extends TestRendererSetup {
  app: App;
  store: DrawingStore;
  copied: string[];
  quit: { called: boolean };
  frame(): Promise<string>;
  /** Screen position of a toolbar label. */
  label(text: string): Promise<{ x: number; y: number }>;
  click(x: number, y: number): Promise<void>;
  drag(x0: number, y0: number, x1: number, y1: number, button?: 0 | 1): Promise<void>;
  /** Waits for queued input: mouse-ups arrive a tick late, a lone Esc after the escape timeout. */
  settle(): Promise<void>;
  escape(): Promise<void>;
  /** Background colors the app has applied, in order; the first is the one the first frame used. */
  themes: string[];
}

let dir: string;
let current: Harness | null = null;

async function start(
  width = 120,
  height = 40,
  layer = new Layer(),
  term: TerminalColors | null = null,
): Promise<Harness> {
  dir = mkdtempSync(join(tmpdir(), "lazydraw-test-"));
  // `exitOnCtrlC: false` as in cli.ts: ctrl+c is the app's quit binding, not the renderer's.
  const setup = await createTestRenderer({
    width,
    height,
    useMouse: true,
    enableMouseMovement: true,
    exitOnCtrlC: false,
  });
  const store = new DrawingStore(join(dir, "drawings"));
  const path = store.pathFor("test");
  store.save(path, "test", layer, "2026-01-01T00:00:00.000Z");
  const copied: string[] = [];
  const quit = { called: false };
  const themes: string[] = [];
  const setBg = setup.renderer.setBackgroundColor.bind(setup.renderer);
  setup.renderer.setBackgroundColor = (c) => {
    themes.push(String(c));
    setBg(c);
  };
  const app = new App({
    renderer: setup.renderer,
    store,
    config: structuredClone(DEFAULT_CONFIG),
    configPath: join(dir, "config.json"),
    drawing: { path, name: "test", layer, createdAt: "2026-01-01T00:00:00.000Z" },
    clipboard: { copy: (t) => void copied.push(t), paste: () => "PASTED" },
    term,
    onQuit: () => (quit.called = true),
    autosaveMs: 20,
  });
  const frame = async () => {
    await setup.renderOnce();
    // The lattice glyph U+1FB7C is a surrogate pair; swap it for a one-unit stand-in so string indices are columns.
    return setup.captureCharFrame().replaceAll("\u{1FB7C}", "▏");
  };
  const h: Harness = {
    ...setup,
    app,
    store,
    copied,
    quit,
    themes,
    frame,
    async label(text) {
      const lines = (await frame()).split("\n");
      const y = BAR_Y + 1;
      const x = lines[y]!.indexOf(text);
      return { x, y };
    },
    async click(x, y) {
      await setup.mockMouse.click(x, y);
      await h.settle();
    },
    async drag(x0, y0, x1, y1, button = 0) {
      await setup.mockMouse.drag(x0, y0, x1, y1, button);
      await h.settle();
    },
    async settle() {
      await Bun.sleep(15);
      await setup.renderOnce();
    },
    async escape() {
      setup.mockInput.pressEscape();
      await Bun.sleep(60);
      await setup.renderOnce();
    },
  };
  current = h;
  await frame();
  return h;
}

afterEach(() => {
  current?.app.shutdown();
  current?.renderer.destroy();
  current = null;
  rmSync(dir, { recursive: true, force: true });
});

/** Canvas cell under a screen position. */
const cellAt = (h: Harness, x: number, y: number) => h.app.viewport.toCanvas(x, y);

describe("toolbar", () => {
  // 77 cells: files/settings/undo/redo became single-glyph icons (≡/⟲/⟳/⚙), shrinking their groups.
  test("full layout is 77 cells wide with ASCIIFlow's order", async () => {
    const h = await start(120);
    const lines = (await h.frame()).split("\n");
    expect(lines[2]).toContain(
      "│  ≡  │  box select arrow line text eraser  │  export  │  ⟲ ⟳  │  ⚙  │  help  │",
    );
    const x0 = lines[1]!.indexOf("┌");
    const x1 = lines[1]!.indexOf("┐");
    expect(x1 - x0 + 1).toBe(79);
    expect(x0).toBe(Math.floor((120 - 79) / 2));
  });

  test("compact and narrow layouts", async () => {
    let h = await start(65);
    expect((await h.frame()).split("\n")[2]).toContain(
      "│ ≡ │ box sel arrow line text erase │ exp │ ⟲ ⟳ │ ⚙ │ ? │",
    );
    h.app.shutdown();
    h.renderer.destroy();
    h = await start(38);
    current = h;
    expect((await h.frame()).split("\n")[2]).toContain("│ ≡ │ box sel arw lin txt ers │");
  });

  test("clicking tools switches and highlights them; digits and alt+digits too", async () => {
    const h = await start();
    const arrow = await h.label("arrow");
    await h.click(arrow.x, arrow.y);
    expect(h.app.tool).toBe("arrow");
    h.mockInput.pressKey("2");
    expect(h.app.tool).toBe("select");
    h.mockInput.pressKey("5", { meta: true });
    expect(h.app.tool).toBe("text");
    const spans = h.captureSpans();
    const row = spans.lines[2]!;
    const text = row.spans.find((s) => s.text.includes("text") && s.text.trim() === "text");
    expect(text).toBeDefined();
  });

  test("letter shortcuts switch tools: r v a l t e", async () => {
    const h = await start();
    for (const [key, tool] of [
      ["v", "select"],
      ["a", "arrow"],
      ["l", "line"],
      ["t", "text"],
      ["e", "eraser"],
      ["r", "box"],
    ] as const) {
      h.mockInput.pressKey(key);
      expect(h.app.tool).toBe(tool);
    }
  });

  test("no title above the bar; the status bar shows the product name instead of coordinates", async () => {
    const h = await start();
    const lines = (await h.frame()).split("\n");
    expect(lines[BAR_Y - 1]).not.toContain("lazydraw");
    expect(lines[lines.length - 2]).toContain("lazydraw");
  });

  test("popovers open under their labels and close on escape or outside click", async () => {
    const h = await start();
    const settings = await h.label("⚙");
    await h.click(settings.x, settings.y);
    let f = await h.frame();
    expect(f).toContain("lattice");
    expect(f.split("\n")[5]).toContain("grid:");
    // The eleven themes wrap onto three rows, all inside the panel.
    const rows = f.split("\n").slice(6, 9).map((l) => l.slice(l.indexOf("│") + 1, l.lastIndexOf("│")));
    expect(rows[0]).toContain("theme: terminal  dracula  nord  tokyo-night");
    expect(rows.join(" ")).toContain("github-light");
    for (const row of rows) expect(row.length).toBeLessThanOrEqual(56);
    await h.escape();
    expect(await h.frame()).not.toContain("lattice");
    await h.click(settings.x, settings.y);
    await h.click(5, 30);
    f = await h.frame();
    expect(f).not.toContain("lattice");
    expect(h.app.editor.canvas.committed.size()).toBe(0);
  });
});

describe("theme", () => {
  const TERM: TerminalColors = { fg: "#00ff00", bg: "#110022", ansi: Array(16).fill("#00ff00") };

  test("the first frame already wears the terminal's colors, with no flash of the stand-in", async () => {
    const h = await start(80, 24, new Layer(), TERM);
    expect(h.themes[0]).toBe(String(palette("terminal", TERM).bg));
    expect(h.themes).not.toContain(String(palette("terminal", null).bg));
  });

  test("a terminal that has not answered yet gets the stand-in until it does", async () => {
    const h = await start(80, 24);
    expect(h.themes[0]).toBe(String(palette("terminal", null).bg));
  });
});

describe("drawing", () => {
  test("box drag draws, undo/redo buttons work", async () => {
    const h = await start();
    await h.drag(10, 10, 19, 14);
    await h.renderOnce();
    const f = (await h.frame()).split("\n");
    expect(f[10]!.slice(10, 20)).toBe("┌────────┐");
    expect(f[14]!.slice(10, 20)).toBe("└────────┘");
    expect(h.app.editor.canvas.committed.size()).toBe(26);

    const undo = await h.label("⟲");
    await h.click(undo.x, undo.y);
    expect(h.app.editor.canvas.committed.size()).toBe(0);
    const redo = await h.label("⟳");
    await h.click(redo.x, redo.y);
    expect(h.app.editor.canvas.committed.size()).toBe(26);
  });

  test("drags starting on the toolbar are ignored", async () => {
    const h = await start();
    await h.drag(25, 2, 30, 20);
    expect(h.app.editor.canvas.committed.size()).toBe(0);
  });

  test("arrow from a box side, flipped mid-drag with f", async () => {
    const h = await start();
    await h.drag(10, 10, 16, 14);
    h.mockInput.pressKey("3");
    const m = h.mockMouse;
    await m.pressDown(16, 12);
    await m.emitMouseEvent("drag", 20, 16, 0);
    await m.emitMouseEvent("drag", 24, 18, 0);
    await h.settle();
    const before = h.app.editor.canvas.scratch.get(cellAt(h, 24, 12));
    h.mockInput.pressKey("f");
    const after = h.app.editor.canvas.scratch.get(cellAt(h, 16, 18));
    await m.release(24, 18);
    await h.settle();
    expect(before).toBe("┐");
    expect(after).toBe("└");
    const f = (await h.frame()).split("\n");
    expect(f[18]!.slice(16, 25)).toBe("└───────►");
  });

  test("wheel pans the viewport; grid stays anchored to the canvas", async () => {
    const h = await start();
    const o = h.app.viewport.origin;
    await h.mockMouse.scroll(50, 20, "down");
    expect(h.app.viewport.origin.y).toBe(o.y + 1);
    await h.mockMouse.scroll(50, 20, "right");
    expect(h.app.viewport.origin.x).toBe(o.x + 2);
  });

  test("middle-drag pans", async () => {
    const h = await start();
    const o = h.app.viewport.origin;
    await h.drag(50, 20, 45, 17, 1);
    expect(h.app.viewport.origin.x).toBe(o.x + 5);
    expect(h.app.viewport.origin.y).toBe(o.y + 3);
    expect(h.app.editor.canvas.committed.size()).toBe(0);
  });

  test("text: undo while typing only removes keystrokes (v1.1 regression)", async () => {
    const h = await start();
    await h.drag(10, 10, 16, 14);
    h.mockInput.pressKey("5");
    await h.click(30, 20);
    await h.mockInput.typeText("abc");
    h.mockInput.pressKey("z", { ctrl: true });
    const rendered = h.app.editor.canvas.rendered();
    expect(rendered.get(cellAt(h, 30, 20))).toBe("a");
    expect(rendered.get(cellAt(h, 31, 20))).toBe("b");
    expect(rendered.get(cellAt(h, 32, 20))).toBeNull();
    expect(h.app.editor.canvas.committed.size()).toBe(20);
    await h.escape();
    expect(h.app.editor.canvas.committed.size()).toBe(22);
    h.mockInput.pressKey("z", { ctrl: true });
    expect(h.app.editor.canvas.committed.size()).toBe(20);
    h.mockInput.pressKey("z", { ctrl: true });
    expect(h.app.editor.canvas.committed.size()).toBe(0);
  });

  test("text: enter returns to the start column; digits are text, not shortcuts", async () => {
    const h = await start();
    h.mockInput.pressKey("5");
    await h.click(30, 20);
    await h.mockInput.typeText("1");
    h.mockInput.pressEnter();
    await h.mockInput.typeText("2");
    await h.escape();
    expect(h.app.tool).toBe("text");
    const c = h.app.editor.canvas.committed;
    expect(c.get(cellAt(h, 30, 20))).toBe("1");
    expect(c.get(cellAt(h, 30, 21))).toBe("2");
  });

  test("select: copy, cut, paste and nudge", async () => {
    const h = await start();
    await h.drag(10, 10, 14, 12);
    h.mockInput.pressKey("2");
    await h.drag(8, 8, 16, 13);
    h.mockInput.pressKey("c", { ctrl: true });
    expect(h.copied.at(-1)).toContain("┌───┐");
    h.mockInput.pressArrow("right");
    expect(h.app.editor.canvas.committed.get(cellAt(h, 11, 10))).toBe("┌");
    h.mockInput.pressKey("x", { ctrl: true });
    expect(h.app.editor.canvas.committed.size()).toBe(0);
    h.mockInput.pressKey("v", { ctrl: true });
    expect(h.app.editor.canvas.committed.size()).toBe(6);
  });

  test("bracketed paste lands on the canvas", async () => {
    const h = await start();
    await h.mockInput.pasteBracketedText("+--+\n|  |\n+--+");
    expect(h.app.editor.canvas.committed.size()).toBe(10);
    expect(h.app.tool).toBe("select");
  });
});

describe("files, export, storage", () => {
  test("autosave writes the drawing", async () => {
    const h = await start();
    await h.drag(10, 10, 14, 12);
    await Bun.sleep(60);
    const saved = JSON.parse(readFileSync(h.app.currentPath, "utf8"));
    expect(saved.cells.length).toBe(12);
    expect(saved.name).toBe("test");
  });

  test("new drawing via dialog, then switch back from the list", async () => {
    const h = await start();
    await h.drag(10, 10, 14, 12);
    const files = await h.label("≡");
    await h.click(files.x, files.y);
    let f = await h.frame();
    expect(f).toContain("> test");
    let lines = f.split("\n");
    let y = lines.findIndex((l) => l.includes("[new]"));
    await h.click(lines[y]!.indexOf("[new]") + 1, y);
    expect(await h.frame()).toContain("new drawing");
    h.mockInput.pressKey("u", { ctrl: true });
    await h.mockInput.typeText("second");
    h.mockInput.pressEnter();
    await h.settle();
    expect(h.app.drawingName).toBe("second");
    expect(h.app.editor.canvas.committed.size()).toBe(0);

    await h.click(files.x, files.y);
    lines = (await h.frame()).split("\n");
    y = lines.findIndex((l) => l.includes("  test "));
    await h.click(lines[y]!.indexOf("test"), y);
    expect(h.app.drawingName).toBe("test");
    expect(h.app.editor.canvas.committed.size()).toBe(12);
  });

  test("rename rejects duplicates", async () => {
    const h = await start();
    h.store.create("taken");
    h.app.renameDrawing();
    h.mockInput.pressKey("u", { ctrl: true });
    await h.mockInput.typeText("taken");
    h.mockInput.pressEnter();
    await h.settle();
    expect(await h.frame()).toContain("a drawing with that name exists");
    await h.escape();
    expect(h.app.drawingName).toBe("test");
  });

  test("export dialog previews, switches charset and copies", async () => {
    const h = await start();
    await h.drag(10, 10, 14, 12);
    h.mockInput.pressKey("e", { ctrl: true });
    let f = await h.frame();
    expect(f).toContain("[copy to clipboard]");
    expect(f).toContain("┌───┐");
    const lines = f.split("\n");
    const y = lines.findIndex((l) => l.includes("basic"));
    await h.click(lines[y]!.indexOf("basic"), y);
    f = await h.frame();
    expect(f).toContain("+---+");
    const hy = f.split("\n").findIndex((l) => l.includes("# hash"));
    await h.click(f.split("\n")[hy]!.indexOf("# hash"), hy);
    const cy = (await h.frame()).split("\n").findIndex((l) => l.includes("[copy to clipboard]"));
    await h.click((await h.frame()).split("\n")[cy]!.indexOf("[copy") + 1, cy);
    expect(h.copied.at(-1)).toBe("# +---+\n# |   |\n# +---+");
  });

  test("settings: grid style toggles live", async () => {
    const h = await start();
    expect(await h.frame()).toContain("▏");
    h.app.setGrid("dots");
    const f = await h.frame();
    expect(f).not.toContain("▏");
    expect(f).toContain("·");
  });

  test("help carries no attribution line; the README credits instead", async () => {
    const h = await start();
    const help = await h.label("help");
    await h.click(help.x, help.y);
    const f = await h.frame();
    expect(f).toContain("switch tool");
    expect(f).not.toContain("asciiflow");
    expect(f).not.toContain("github.com");
    expect(f).not.toContain("a terminal take on");
  });

  test("notices sit where the quit prompt does, left of the drawing name", async () => {
    const h = await start();
    h.mockInput.pressKey("s", { ctrl: true });
    const lines = (await h.frame()).split("\n");
    const row = lines.find((l) => l.includes("saved"))!;
    expect(row).toBeDefined();
    // Left edge, like the quit prompt; the name/position stay on the right.
    expect(row.indexOf("saved")).toBeLessThan(4);
    expect(row.lastIndexOf("test")).toBeGreaterThan(row.length / 2);
    // And the tool hint it displaces is gone while the notice shows.
    expect(row).not.toContain("drag to draw a box");
  });

  test("the quit prompt outranks a toast in that slot", async () => {
    const h = await start();
    h.mockInput.pressKey("s", { ctrl: true });
    expect(await h.frame()).toContain("saved");
    h.mockInput.pressKey("q", { ctrl: true });
    const f = await h.frame();
    expect(f).toContain("press ctrl+q again to exit");
    expect(f).not.toContain("saved");
  });

  test("bare q no longer quits", async () => {
    const h = await start();
    h.mockInput.pressKey("q");
    expect(h.quit.called).toBe(false);
  });

  test("ctrl+q prompts first, then quits on the second press", async () => {
    const h = await start();
    h.mockInput.pressKey("q", { ctrl: true });
    expect(h.quit.called).toBe(false);
    expect(await h.frame()).toContain("press ctrl+q again to exit");
    h.mockInput.pressKey("q", { ctrl: true });
    expect(h.quit.called).toBe(true);
  });

  test("settings: the copy-on-select toggle shows and flips", async () => {
    const h = await start();
    const settings = await h.label("⚙");
    await h.click(settings.x, settings.y);
    let f = await h.frame();
    expect(f).toContain("copy on select:");
    expect(f).toMatch(/copy on select:\s+off/);
    expect(h.app.config.copyOnSelect).toBe(false);
    const lines = f.split("\n");
    const y = lines.findIndex((l) => l.includes("copy on select:"));
    await h.click(lines[y]!.indexOf("off"), y);
    f = await h.frame();
    expect(f).toMatch(/copy on select:\s+on/);
    expect(h.app.config.copyOnSelect).toBe(true);
  });

  test("copy on select is off by default", async () => {
    const h = await start();
    await h.drag(10, 10, 14, 12);
    h.mockInput.pressKey("2");
    await h.drag(8, 8, 16, 13);
    expect(h.copied.length).toBe(0);
  });

  test("the settings toggle makes a finished selection copy itself", async () => {
    const h = await start();
    h.app.setCopyOnSelect(true);
    await h.drag(10, 10, 14, 12);
    h.mockInput.pressKey("2");
    await h.drag(8, 8, 16, 13);
    expect(h.copied.at(-1)).toContain("┌───┐");
    // Re-selecting the same cells yields the same text; the clipboard is left alone.
    const n = h.copied.length;
    await h.drag(8, 8, 16, 13);
    expect(h.copied.length).toBe(n);
  });

  test("the prompt lapses after 3s, so a later ctrl+q only re-arms", async () => {
    const h = await start();
    h.mockInput.pressKey("q", { ctrl: true });
    await Bun.sleep(3010);
    expect(await h.frame()).not.toContain("press ctrl+q again to exit");
    h.mockInput.pressKey("q", { ctrl: true });
    expect(h.quit.called).toBe(false);
    expect(await h.frame()).toContain("press ctrl+q again to exit");
  });
});

describe("space to pan", () => {
  test("space + drag pans in drawing tools", async () => {
    const h = await start();
    const o = h.app.viewport.origin;
    h.mockInput.pressKey(" ");
    await h.drag(50, 20, 40, 20);
    expect(h.app.viewport.origin.x).toBe(o.x + 10);
    expect(h.app.editor.canvas.committed.size()).toBe(0);
  });

  test("in text mode a single typed space doesn't pan; a held one does and is taken back", async () => {
    const h = await start();
    h.mockInput.pressKey("5");
    await h.click(30, 20);
    await h.mockInput.typeText("a ");
    await h.click(40, 25);
    expect(h.app.editor.text.cursor).toEqual(cellAt(h, 40, 25));
    await h.mockInput.typeText("b");
    const o = h.app.viewport.origin;
    h.mockInput.pressKey(" ");
    h.mockInput.pressKey(" ");
    h.mockInput.pressKey(" ");
    await h.drag(50, 20, 45, 20);
    expect(h.app.viewport.origin.x).toBe(o.x + 5);
    await h.escape();
    const text = h.app.editor.canvas.committed;
    expect(text.size()).toBe(2);
  });
});

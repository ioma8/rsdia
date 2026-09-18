// The terminal app: one full-screen surface that paints the canvas, toolbar,
// popovers and dialogs, and routes mouse/keyboard input (§3.5, §5, §6).
import { readFileSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { resolve } from "node:path";
import {
  type CliRenderer,
  type KeyEvent,
  type MouseEvent,
  type OptimizedBuffer,
  type PasteEvent,
  Renderable,
  type RenderContext,
} from "@opentui/core";
import { Canvas } from "../core/canvas";
import { Editor, TOOL_IDS, type ToolId } from "../core/editor";
import { type ExportConfig, exportText } from "../core/export";
import type { Layer } from "../core/layer";
import { textSize, textToLayer } from "../core/text";
import { KEY, mods, type HoverHint, type Mods } from "../core/tools/tool";
import { Vector } from "../core/vector";
import { type Config, type GridStyle, saveConfig, type ThemeChoice } from "../storage/config";
import { type DrawingInfo, type DrawingStore, slugify } from "../storage/drawings";
import { renderCanvas, Viewport } from "./canvas-view";
import type { Dialog, Host, InputDialog } from "./host";
import { altDigit, isCtrl, printable, toolKey } from "./input";
import { type Hotspot, hotspotAt, inRect, Painter, type Rect } from "./painter";
import { renderDialog } from "./popovers/dialog";
import { renderExport } from "./popovers/export";
import { renderFiles } from "./popovers/files";
import { renderHelp } from "./popovers/help";
import { renderMenu } from "./popovers/menu";
import { renderSettings } from "./popovers/settings";
import { detectTerminalColors, palette, type Palette, type TerminalColors } from "./theme";
import { type ItemId, layoutToolbar, type PanelId, renderToolbar, type ToolbarHost } from "./toolbar";

export interface OpenDrawing {
  path: string;
  name: string;
  layer: Layer;
  createdAt: string;
}

export interface Clipboard {
  copy(text: string): void;
  paste(): string | null;
}

export interface AppOptions {
  renderer: CliRenderer;
  store: DrawingStore;
  config: Config;
  configPath?: string;
  drawing: OpenDrawing;
  clipboard: Clipboard;
  /** The terminal's colors, if startup already read them; otherwise the app reads them itself. */
  term?: TerminalColors | null;
  onQuit(): void;
  /** Autosave debounce in ms (§7). */
  autosaveMs?: number;
}

const CHIP_MS = 1500;
/** How long a first ctrl+c stays armed, waiting for the second one. */
const QUIT_CONFIRM_MS = 3000;
const TOAST_MS = 2500;
const SPACE_PAN_MS = 1000;

type Mode = { kind: "none" } | { kind: "draw"; last: Vector } | { kind: "pan"; sx: number; sy: number; origin: Vector };

const TOOL_HINT: Record<ToolId, string> = {
  box: "drag to draw a box",
  select: "drag to select or move · del erases · ctrl+c/x/v",
  arrow: "drag to draw an arrow · press f to flip",
  line: "drag to draw a line · press f to flip",
  text: "click to place the cursor, then type",
  eraser: "drag to erase",
};

const TOOL_SHORTCUT: Record<string, ToolId> = {
  r: "box",
  v: "select",
  a: "arrow",
  l: "line",
  t: "text",
  e: "eraser",
};

class Surface extends Renderable {
  constructor(
    ctx: RenderContext,
    private readonly app: App,
  ) {
    super(ctx, {
      id: "lazydraw",
      position: "absolute",
      left: 0,
      top: 0,
      width: "100%",
      height: "100%",
      onMouse: (e) => app.onMouse(e),
    });
  }

  protected override renderSelf(buffer: OptimizedBuffer): void {
    this.app.paint(buffer, this.width, this.height);
  }
}

export class App implements Host, ToolbarHost {
  readonly editor = new Editor();
  readonly viewport = new Viewport();
  readonly config: Config;
  drawings: DrawingInfo[] = [];
  panel: PanelId | null = null;

  private readonly renderer: CliRenderer;
  private readonly store: DrawingStore;
  private readonly opts: AppOptions;
  private readonly surface: Surface;
  private drawing: OpenDrawing;
  private unsubscribe: () => void = () => {};
  private saveTimer: ReturnType<typeof setTimeout> | null = null;
  private dirty = false;

  private panelAnchor = 0;
  private dialog: Dialog | null = null;
  private toastMessage: { text: string; until: number } | null = null;
  private chipsUntil = 0;
  /** When set, a ctrl+c before this moment quits; a first ctrl+c arms it. */
  private quitArmed = 0;
  /** The last text sent to the clipboard, so re-selecting the same cells does not resend it. */
  private lastCopy: string | null = null;
  private hotspots: Hotspot[] = [];
  private chrome: Rect[] = [];
  private pressed: string | null = null;
  private mode: Mode = { kind: "none" };
  private hover: { x: number; y: number } | null = null;
  private hint: HoverHint = "default";
  private flipToggle = false;
  private lastMods: Mods = mods();
  /** Space presses in a row; terminals rarely report releases, so holding is inferred from repeats. */
  private spaceRun = { count: 0, until: 0 };
  private placing: Layer | null = null;
  private width = 0;
  private height = 0;
  private timers = new Set<ReturnType<typeof setTimeout>>();
  /** The terminal's own colors, once detected. */
  private term: TerminalColors | null = null;
  private recenterSoon = true;

  constructor(opts: AppOptions) {
    this.opts = opts;
    this.renderer = opts.renderer;
    this.store = opts.store;
    this.config = opts.config;
    this.drawing = opts.drawing;
    this.term = opts.term ?? null;
    this.attachCanvas(new Canvas(opts.drawing.layer));

    this.surface = new Surface(this.renderer, this);
    this.renderer.root.add(this.surface);
    this.renderer.keyInput.on("keypress", (k) => this.onKey(k));
    this.renderer.keyInput.on("keyrelease", (k) => {
      if (k.name === "space") this.spaceRun = { count: 0, until: 0 };
    });
    this.renderer.keyInput.on("paste", (e) => this.onPaste(e));
    this.renderer.on("resize", () => this.requestRender());
    this.applyTheme();
    if (!this.term) void this.readTerminalColors();
  }

  // ---------------------------------------------------------------- state

  get tool(): ToolId {
    return this.editor.tool;
  }
  get canUndo(): boolean {
    return this.editor.canvas.canUndo || (this.editor.textEntry && this.editor.canvas.scratch.size() > 0);
  }
  get canRedo(): boolean {
    return this.editor.canvas.canRedo;
  }
  get showChips(): boolean {
    return this.panel === "help" || Date.now() < this.chipsUntil;
  }
  get drawingName(): string {
    return this.drawing.name;
  }
  get currentPath(): string {
    return this.drawing.path;
  }

  private get pal(): Palette {
    return palette(this.config.theme, this.term);
  }

  /**
   * Reads the terminal's palette for a theme that inherits it. Startup normally has the answer
   * before the first frame; this covers terminals too slow for that and themes switched to
   * `terminal` later on.
   */
  private async readTerminalColors(): Promise<void> {
    const term = await detectTerminalColors(this.renderer);
    if (!term) return;
    this.term = term;
    this.applyTheme();
  }

  private applyTheme() {
    this.renderer.setBackgroundColor(this.pal.bg);
    this.requestRender();
  }

  requestRender(): void {
    this.renderer.requestRender();
  }

  private later(ms: number, fn: () => void) {
    const t = setTimeout(() => {
      this.timers.delete(t);
      fn();
    }, ms);
    this.timers.add(t);
  }

  toast(text: string): void {
    this.toastMessage = { text, until: Date.now() + TOAST_MS };
    this.later(TOAST_MS + 10, () => this.requestRender());
    this.requestRender();
  }

  private attachCanvas(canvas: Canvas) {
    this.unsubscribe();
    this.editor.setCanvas(canvas);
    this.unsubscribe = canvas.onChange((committed) => {
      if (committed) this.scheduleSave();
      this.requestRender();
    });
  }

  // ---------------------------------------------------------------- saving

  private scheduleSave() {
    this.dirty = true;
    if (this.saveTimer) clearTimeout(this.saveTimer);
    this.saveTimer = setTimeout(() => this.save(), this.opts.autosaveMs ?? 500);
  }

  save(): void {
    if (this.saveTimer) clearTimeout(this.saveTimer);
    this.saveTimer = null;
    try {
      this.store.save(this.drawing.path, this.drawing.name, this.editor.canvas.committed, this.drawing.createdAt);
      this.dirty = false;
    } catch (e) {
      this.toast(`save failed: ${(e as Error).message}`);
    }
  }

  private persistConfig() {
    saveConfig(this.config, this.opts.configPath);
  }

  /** Commits pending edits and writes everything to disk. */
  shutdown(): void {
    this.editor.cancelGesture();
    this.editor.flush();
    if (this.dirty || this.saveTimer) this.save();
    this.config.lastDrawing = this.drawing.path;
    this.persistConfig();
    for (const t of this.timers) clearTimeout(t);
    this.unsubscribe();
  }

  /** Copies a finished selection. Off by default; the `settings` panel turns it on. */
  private autoCopy(): void {
    if (!this.config.copyOnSelect || this.tool !== "select") return;
    const text = this.editor.select.copySelection();
    // Re-selecting or moving the same cells yields the same text; leave the clipboard alone.
    if (!text || text === this.lastCopy) return;
    this.lastCopy = text;
    this.opts.clipboard.copy(text);
  }

  /** First ctrl+q arms the prompt in the status bar; a second one within the window quits. */
  private armQuit(): void {
    if (Date.now() < this.quitArmed) return this.quit();
    this.quitArmed = Date.now() + QUIT_CONFIRM_MS;
    this.later(QUIT_CONFIRM_MS + 10, () => this.requestRender());
  }

  quit(): void {
    this.shutdown();
    this.opts.onQuit();
  }

  // ---------------------------------------------------------------- painting

  paint(buffer: OptimizedBuffer, width: number, height: number): void {
    this.width = width;
    this.height = height;
    const layout = layoutToolbar(width);
    if (this.recenterSoon) {
      this.recenterSoon = false;
      this.viewport.recenter(this.editor, width, height, layout.bottom + 1);
    }
    const p = new Painter(buffer, this.pal, width, height, this.hover);
    const hoverCell = this.hover ? this.viewport.toCanvas(this.hover.x, this.hover.y) : null;
    renderCanvas(p, {
      editor: this.editor,
      viewport: this.viewport,
      grid: this.config.grid,
      hoverCell,
      hoverHint: this.hint,
      cursorOn: true,
    });
    renderToolbar(p, this, layout);
    if (this.panel) {
      const render = {
        files: renderFiles,
        export: renderExport,
        settings: renderSettings,
        help: renderHelp,
        menu: renderMenu,
      }[this.panel];
      render(p, this, this.panelAnchor, layout.bottom);
    }
    this.paintStatus(p);
    if (this.dialog) {
      const before = p.hotspots.length;
      renderDialog(
        p,
        this.dialog,
        () => this.submitDialog(),
        () => this.closeDialog(),
      );
      // The dialog is modal: only its own buttons are live.
      p.hotspots = p.hotspots.slice(before);
    }
    this.hotspots = p.hotspots;
    this.chrome = p.chrome;
  }

  private paintStatus(p: Painter) {
    const { pal } = p;
    const y = this.height - 1;
    if (y < 5) return;
    p.fill({ x: 0, y, w: this.width, h: 1 }, pal.bg);
    let hint = TOOL_HINT[this.tool];
    if (this.placing) hint = "click to place the imported text · esc cancels";
    else if (this.editor.textEntry) hint = "typing · enter: new line · esc: done · ctrl+z: undo keystroke";
    else if (this.editor.drawing && (this.tool === "arrow" || this.tool === "line")) {
      hint = `press f to flip${this.flipToggle ? " · flipped" : ""}`;
    }
    // Every transient notice shares one slot: the hint's place, in the prompt's color. The quit
    // prompt outranks a toast, since it is the one the next keystroke acts on.
    const toast = this.toastMessage && Date.now() < this.toastMessage.until ? this.toastMessage.text : null;
    const message = Date.now() < this.quitArmed ? "press ctrl+q again to exit" : toast;
    const right = `${this.drawing.name}${this.dirty ? " •" : ""}  ·  lazydraw`;
    const rx = Math.max(0, this.width - right.length - 1);
    p.text(1, y, message ?? hint, message ? pal.warning : pal.muted, pal.bg, 0, rx - 2);
    p.text(rx, y, right, pal.muted, pal.bg);
    p.chrome.push({ x: 0, y, w: this.width, h: 1 });
  }

  // ---------------------------------------------------------------- mouse

  private mouseMods(e: MouseEvent): Mods {
    const m = mods({ ctrl: e.modifiers.ctrl, alt: e.modifiers.alt });
    return { ...m, flip: m.flip !== this.flipToggle };
  }

  /** Space is held: once outside text entry, or repeating while typing. */
  private get spaceHeld(): boolean {
    const need = this.editor.textEntry ? 2 : 1;
    return Date.now() < this.spaceRun.until && this.spaceRun.count >= need;
  }

  private overChrome(x: number, y: number): boolean {
    return this.chrome.some((r) => inRect(r, x, y));
  }

  onMouse(e: MouseEvent): void {
    switch (e.type) {
      case "scroll":
        return this.onScroll(e);
      case "move":
      case "drag":
        return this.onMove(e);
      case "down":
        return this.onDown(e);
      case "up":
        return this.onUp(e);
    }
  }

  private onScroll(e: MouseEvent) {
    if (this.dialog || !e.scroll) return;
    if (this.panel && this.overChrome(e.x, e.y)) return;
    const n = Math.max(1, Math.round(e.scroll.delta || 1));
    const dir = e.scroll.direction;
    if (dir === "left" || dir === "right") {
      const sign = dir === "left" ? -1 : 1;
      this.viewport.pan(sign * 2 * n, 0);
    } else {
      this.viewport.pan(0, (dir === "up" ? -1 : 1) * n);
    }
    this.refreshHover(e);
  }

  private refreshHover(e: MouseEvent) {
    this.hover = { x: e.x, y: e.y };
    const cell = this.viewport.toCanvas(e.x, e.y);
    if (this.placing) this.showPlacing(cell);
    this.hint = this.tool === "select" && !this.overChrome(e.x, e.y) ? this.editor.hoverHint(cell, this.mouseMods(e)) : "default";
    this.requestRender();
  }

  private onMove(e: MouseEvent) {
    const mode = this.mode;
    if (mode.kind === "pan") {
      this.viewport.origin = new Vector(mode.origin.x - (e.x - mode.sx), mode.origin.y - (e.y - mode.sy));
    } else if (mode.kind === "draw") {
      const cell = this.viewport.toCanvas(e.x, e.y);
      this.lastMods = this.mouseMods(e);
      if (!cell.equals(mode.last)) {
        mode.last = cell;
        this.editor.move(cell, this.lastMods);
      }
    }
    this.refreshHover(e);
  }

  private onDown(e: MouseEvent) {
    this.hover = { x: e.x, y: e.y };
    const hot = hotspotAt(this.hotspots, e.x, e.y);
    this.pressed = hot?.id ?? null;
    if (hot || this.dialog) return;
    if (this.overChrome(e.x, e.y)) return;
    if (this.panel) {
      // A click outside a popover only closes it.
      this.closePanel();
      return;
    }
    if (this.mode.kind !== "none") return;
    const cell = this.viewport.toCanvas(e.x, e.y);
    if (e.button === 1 || (e.button === 0 && this.spaceHeld)) {
      if (e.button === 0 && this.editor.textEntry) {
        // The held space was typed into the text; take it back.
        for (let i = 0; i < this.spaceRun.count && this.editor.text.lastTypedSpace; i++) {
          this.editor.text.undoKeystroke();
        }
      }
      this.mode = { kind: "pan", sx: e.x, sy: e.y, origin: this.viewport.origin };
      return;
    }
    if (e.button !== 0) return;
    if (this.placing) {
      this.showPlacing(cell);
      this.editor.canvas.commitScratch();
      this.placing = null;
      this.toast("imported");
      return;
    }
    this.flipToggle = false;
    this.lastMods = this.mouseMods(e);
    this.mode = { kind: "draw", last: cell };
    this.editor.down(cell, this.lastMods);
    this.requestRender();
  }

  private onUp(e: MouseEvent) {
    const pressed = this.pressed;
    this.pressed = null;
    if (pressed) {
      const hot = hotspotAt(this.hotspots, e.x, e.y);
      if (hot?.id === pressed) hot.action();
      this.requestRender();
      return;
    }
    if (this.mode.kind === "draw") {
      this.editor.up();
      this.autoCopy();
    }
    this.mode = { kind: "none" };
    this.flipToggle = false;
    this.refreshHover(e);
  }

  // ---------------------------------------------------------------- keyboard

  private onKey(k: KeyEvent) {
    try {
      this.handleKey(k);
    } finally {
      this.requestRender();
    }
  }

  private handleKey(k: KeyEvent) {
    if (this.dialog) return this.dialogKey(k);

    // Global shortcuts.
    if (isCtrl(k, "q")) return this.armQuit();
    if (isCtrl(k, "c")) return this.copySelection(false);
    if (isCtrl(k, "z")) return void (k.shift ? this.redo() : this.undo());
    if (isCtrl(k, "y")) return void this.redo();
    if (isCtrl(k, "s")) {
      this.editor.flush();
      this.save();
      return this.toast("saved");
    }
    if (isCtrl(k, "e")) return this.togglePanel("export", this.anchorOf("export"));
    if (isCtrl(k, "o")) return this.togglePanel("files", this.anchorOf("files"));
    if (isCtrl(k, "x")) return this.copySelection(true);
    if (isCtrl(k, "v")) {
      const text = this.opts.clipboard.paste();
      if (text) this.pasteText(text);
      return;
    }

    if (k.name === "escape") return this.escape();

    const digit = altDigit(k);
    if (digit) return this.setTool(TOOL_IDS[digit - 1]!);

    const key = toolKey(k);

    // Mid-drag: `f` flips line/arrow/select elbows; other keys go to the tool.
    if (this.editor.drawing && this.mode.kind === "draw") {
      if (key === "f") {
        this.flipToggle = !this.flipToggle;
        this.lastMods = { ...this.lastMods, flip: !this.lastMods.flip };
        if (this.tool === "select") this.editor.move(this.mode.last, this.lastMods);
        else this.editor.key("", this.lastMods);
        return;
      }
      if (key) this.editor.key(key, this.lastMods);
      return;
    }

    if (k.name === "space") {
      const now = Date.now();
      this.spaceRun = { count: now < this.spaceRun.until ? this.spaceRun.count + 1 : 1, until: now + SPACE_PAN_MS };
      if (!this.editor.textEntry) return;
    } else {
      this.spaceRun = { count: 0, until: 0 };
    }

    if (this.editor.textEntry) {
      if (key) this.editor.key(key, mods());
      return;
    }

    const ch = printable(k);
    if (ch && /^[1-6]$/.test(ch)) return this.setTool(TOOL_IDS[Number(ch) - 1]!);
    const letterTool = ch ? TOOL_SHORTCUT[ch] : undefined;
    if (letterTool) return this.setTool(letterTool);
    if (ch === "?") return this.togglePanel("help", this.anchorOf("help"));

    if (this.tool === "select" && key) {
      this.editor.key(key, mods());
      return;
    }
  }

  private escape() {
    if (this.panel) return this.closePanel();
    if (this.placing) {
      this.placing = null;
      this.editor.canvas.clearScratch();
      return;
    }
    if (this.mode.kind === "draw") {
      this.editor.cancelGesture();
      this.mode = { kind: "none" };
      return;
    }
    if (this.editor.textEntry) return this.editor.text.commit();
    if (this.editor.canvas.selection) this.editor.select.cleanup();
  }

  private dialogKey(k: KeyEvent) {
    const d = this.dialog!;
    if (k.name === "escape") return this.closeDialog();
    if (k.name === "return" || k.name === "enter") return this.submitDialog();
    if (d.kind !== "input") {
      if (k.name === "y") return this.submitDialog();
      if (k.name === "n") this.closeDialog();
      return;
    }
    const chars = [...d.value];
    switch (k.name) {
      case "backspace":
        if (d.cursor > 0) chars.splice(--d.cursor, 1);
        break;
      case "delete":
        chars.splice(d.cursor, 1);
        break;
      case "left":
        d.cursor = Math.max(0, d.cursor - 1);
        break;
      case "right":
        d.cursor = Math.min(chars.length, d.cursor + 1);
        break;
      case "home":
        d.cursor = 0;
        break;
      case "end":
        d.cursor = chars.length;
        break;
      default: {
        if (isCtrl(k, "a")) d.cursor = 0;
        else if (isCtrl(k, "e")) d.cursor = chars.length;
        else if (isCtrl(k, "u")) {
          chars.splice(0, d.cursor);
          d.cursor = 0;
        } else {
          const ch = printable(k);
          if (ch) chars.splice(d.cursor++, 0, ch);
        }
      }
    }
    d.value = chars.join("");
    d.error = null;
  }

  private onPaste(e: PasteEvent) {
    const text = new TextDecoder().decode(e.bytes);
    if (this.dialog?.kind === "input") {
      const d = this.dialog;
      const insert = text.split(/\r?\n/)[0] ?? "";
      const chars = [...d.value];
      chars.splice(d.cursor, 0, ...insert);
      d.value = chars.join("");
      d.cursor += [...insert].length;
    } else if (this.editor.textEntry) {
      for (const ch of text.replace(/\r\n?/g, "\n")) {
        this.editor.key(ch === "\n" ? KEY.ENTER : ch, mods());
      }
    } else if (!this.dialog) {
      this.pasteText(text);
    }
    this.requestRender();
  }

  // ---------------------------------------------------------------- actions

  undo(): void {
    this.editor.undo();
    this.requestRender();
  }

  redo(): void {
    this.editor.redo();
    this.requestRender();
  }

  setTool(id: ToolId): void {
    this.editor.setTool(id);
    this.hint = "default";
    this.chipsUntil = Date.now() + CHIP_MS;
    this.later(CHIP_MS + 10, () => this.requestRender());
    this.requestRender();
  }

  private anchorOf(id: ItemId): number {
    return layoutToolbar(this.width).items.find((i) => i.id === id)?.x ?? 1;
  }

  private togglePanel(id: PanelId, anchorX: number) {
    if (this.panel === id) return this.closePanel();
    if (id === "files") this.drawings = this.store.list();
    this.panel = id;
    this.panelAnchor = anchorX;
    this.requestRender();
  }

  closePanel(): void {
    this.panel = null;
    this.requestRender();
  }

  activate(id: ItemId, anchorX: number): void {
    switch (id) {
      case "quit":
        this.quit();
        break;
      case "undo":
        this.undo();
        break;
      case "redo":
        this.redo();
        break;
      case "files":
      case "export":
      case "settings":
      case "help":
      case "menu":
        this.togglePanel(id, id === "menu" ? anchorX : this.anchorOf(id));
        break;
      default:
        this.setTool(id);
        this.panel = null;
    }
    this.requestRender();
  }

  private copySelection(cut: boolean) {
    if (this.tool !== "select") return;
    const text = this.editor.select.copySelection();
    if (!text) return;
    this.lastCopy = text;
    this.opts.clipboard.copy(text);
    if (cut) this.editor.select.eraseSelection();
    this.toast(cut ? "cut to clipboard" : "copied to clipboard");
  }

  private pasteText(text: string) {
    const size = textSize(text);
    const select = this.editor.select;
    const at =
      this.tool === "select" && select.hasSelection && select.selectBox
        ? select.selectBox.topLeft()
        : this.viewport.toCanvas(
            Math.floor((this.width - size.x) / 2),
            Math.floor((this.height - size.y) / 2),
          );
    this.editor.cancelGesture();
    this.setTool("select");
    this.editor.select.paste(text, at);
    this.toast("pasted");
  }

  // dialogs

  openDialog(d: Dialog): void {
    this.dialog = d;
    this.requestRender();
  }

  closeDialog(): void {
    this.dialog = null;
    this.requestRender();
  }

  private async submitDialog() {
    const d = this.dialog;
    if (!d) return;
    if (d.kind === "confirm") {
      this.dialog = null;
      d.confirm();
      this.requestRender();
      return;
    }
    const error = await d.submit(d.value.trim());
    if (this.dialog !== d) return;
    if (error) d.error = error;
    else this.dialog = null;
    this.requestRender();
  }

  private prompt(title: string, label: string, value: string, submit: InputDialog["submit"]) {
    this.openDialog({ kind: "input", title, label, value, cursor: [...value].length, error: null, submit });
  }

  private nameError(name: string, except?: string): string | null {
    if (!name) return "name can't be empty";
    if (slugify(name) !== (except && slugify(except)) && this.store.exists(name)) return "a drawing with that name exists";
    return null;
  }

  // files

  openDrawing(path: string): void {
    if (path === this.drawing.path) return this.closePanel();
    try {
      const { file, layer } = this.store.loadSync(path);
      this.switchTo({ path, name: file.name, layer, createdAt: file.createdAt });
    } catch (e) {
      this.toast(`can't open: ${(e as Error).message}`);
    }
  }

  private switchTo(d: OpenDrawing) {
    this.editor.cancelGesture();
    this.editor.flush();
    if (this.dirty || this.saveTimer) this.save();
    this.placing = null;
    this.drawing = d;
    this.attachCanvas(new Canvas(d.layer));
    this.config.lastDrawing = d.path;
    this.persistConfig();
    this.panel = null;
    this.recenterSoon = true;
    this.requestRender();
  }

  newDrawing(): void {
    this.prompt("new drawing", "name", this.store.uniqueName(), (name) => {
      const err = this.nameError(name);
      if (err) return err;
      const path = this.store.create(name);
      this.switchTo({ path, name, layer: new Canvas().committed, createdAt: new Date().toISOString() });
      return null;
    });
  }

  renameDrawing(): void {
    this.prompt("rename drawing", "name", this.drawing.name, (name) => {
      if (name === this.drawing.name) return null;
      const err = this.nameError(name, this.drawing.name);
      if (err) return err;
      this.editor.flush();
      const path = this.store.rename(this.drawing.path, name, this.editor.canvas.committed, this.drawing.createdAt);
      this.drawing = { ...this.drawing, path, name };
      this.dirty = false;
      this.drawings = this.store.list();
      this.config.lastDrawing = path;
      this.persistConfig();
      return null;
    });
  }

  forkDrawing(): void {
    this.prompt("fork drawing", "name", this.store.uniqueName(`${this.drawing.name} copy`), (name) => {
      const err = this.nameError(name);
      if (err) return err;
      this.editor.flush();
      const layer = this.editor.canvas.committed.clone();
      const path = this.store.pathFor(name);
      const createdAt = new Date().toISOString();
      this.store.save(path, name, layer, createdAt);
      this.switchTo({ path, name, layer, createdAt });
      this.toast(`forked to ${name}`);
      return null;
    });
  }

  deleteDrawing(): void {
    this.openDialog({
      kind: "confirm",
      title: "delete drawing",
      message: `delete "${this.drawing.name}"? this can't be undone.`,
      yes: "delete",
      confirm: () => {
        // Settle pending edits first so autosave can't recreate the file.
        this.editor.cancelGesture();
        this.editor.flush();
        if (this.saveTimer) clearTimeout(this.saveTimer);
        this.saveTimer = null;
        this.dirty = false;
        this.store.delete(this.drawing.path);
        const next = this.store.list()[0];
        if (next) {
          const { file, layer } = this.store.loadSync(next.path);
          this.switchTo({ path: next.path, name: file.name, layer, createdAt: file.createdAt });
        } else {
          const name = this.store.uniqueName();
          const path = this.store.create(name);
          this.switchTo({ path, name, layer: new Canvas().committed, createdAt: new Date().toISOString() });
        }
        this.drawings = this.store.list();
        this.toast("deleted");
      },
    });
  }

  clearDrawing(): void {
    this.editor.cancelGesture();
    this.editor.flush();
    this.editor.select.cleanup();
    this.editor.canvas.clear();
    this.panel = null;
    this.toast("cleared · ctrl+z to undo");
  }

  importText(): void {
    this.prompt("import text", "file", "", (value) => {
      const path = resolve(value.replace(/^~(?=$|\/)/, homedir()));
      let text: string;
      try {
        text = readFileSync(path, "utf8");
      } catch {
        return "can't read that file";
      }
      this.startPlacing(text);
      return null;
    });
  }

  /** Imported text follows the pointer as scratch until a click commits it. */
  startPlacing(text: string): void {
    const layer = textToLayer(text);
    if (layer.size() === 0) return this.toast("nothing to import");
    this.editor.cancelGesture();
    this.editor.flush();
    this.panel = null;
    this.placing = layer;
    const size = textSize(text);
    const center = this.viewport.toCanvas(
      Math.floor((this.width - size.x) / 2),
      Math.floor((this.height - size.y) / 2),
    );
    this.showPlacing(center);
  }

  private showPlacing(at: Vector) {
    if (!this.placing) return;
    const scratch = new Canvas().committed;
    for (const [p, v] of this.placing.entries()) scratch.set(p.add(at), v);
    this.editor.canvas.setScratch(scratch);
  }

  // export

  setExport(patch: Partial<ExportConfig>): void {
    this.config.export = { ...this.config.export, ...patch };
    this.persistConfig();
    this.requestRender();
  }

  exportPreview(): string {
    return exportText(this.editor.canvas.committed, this.config.export) || "(empty drawing)";
  }

  copyExport(): void {
    this.opts.clipboard.copy(exportText(this.editor.canvas.committed, this.config.export));
    this.toast("copied to clipboard");
  }

  saveExport(): void {
    this.prompt("save export", "file", `${slugify(this.drawing.name)}.txt`, (value) => {
      const path = resolve(value.replace(/^~(?=$|\/)/, homedir()));
      try {
        writeFileSync(path, `${exportText(this.editor.canvas.committed, this.config.export)}\n`);
      } catch (e) {
        return `can't write: ${(e as Error).message}`;
      }
      this.toast(`saved ${path}`);
      return null;
    });
  }

  // settings

  setGrid(g: GridStyle): void {
    this.config.grid = g;
    this.persistConfig();
    this.requestRender();
  }

  setCopyOnSelect(on: boolean): void {
    this.config.copyOnSelect = on;
    this.persistConfig();
    this.requestRender();
  }

  setTheme(t: ThemeChoice): void {
    this.config.theme = t;
    this.persistConfig();
    this.applyTheme();
  }

  recenter(): void {
    this.recenterSoon = true;
    this.requestRender();
  }
}

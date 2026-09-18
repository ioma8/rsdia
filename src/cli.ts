#!/usr/bin/env bun
// lazydraw — terminal ASCII diagram editor.
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { basename, resolve } from "node:path";
import { parseArgs } from "node:util";
import { type CliRenderer, createCliRenderer } from "@opentui/core";
import pkg from "../package.json";
import { type ExportConfig, exportText, isWrapper, type Wrapper } from "./core/export";
import { Layer } from "./core/layer";
import { textToLayer } from "./core/text";
import { loadConfig, saveConfig } from "./storage/config";
import { DrawingStore, FILE_EXT } from "./storage/drawings";
import { App, type Clipboard, type OpenDrawing } from "./tui/app";
import { detectTerminalColorsSoon, PALETTE_WAIT_MS } from "./tui/theme";

const HELP = `lazydraw ${pkg.version} — terminal ASCII diagram editor

usage:
  lazydraw                         open the last drawing (or "untitled")
  lazydraw <name|path>             open or create a drawing
  lazydraw --import diagram.txt    new drawing from a text file
  lazydraw export <name|path> [--basic] [--comment <style>] [--fenced] [-o out.txt]
  lazydraw list                    list saved drawings
  lazydraw --version | --help

comment styles: none standard filled quotes hashes slashes three-slashes
                dashes apostrophes backticks four-spaces semicolons

drawings live in ${new DrawingStore().dir}`;

const COMMENT_ALIASES: Record<string, Wrapper> = {
  standard: "star",
  filled: "star-filled",
  quotes: "triple-quotes",
  hashes: "hash",
  slashes: "slash",
  dashes: "dash",
  apostrophes: "apostrophe",
  semicolons: "semicolon",
};

function fail(message: string): never {
  console.error(`lazydraw: ${message}`);
  process.exit(1);
}

const isPathLike = (arg: string) => arg.includes("/") || arg.endsWith(FILE_EXT) || arg.endsWith(".json");

/** Resolves a CLI argument to a drawing file path, if one exists. */
function findDrawing(store: DrawingStore, arg: string): string | null {
  if (isPathLike(arg)) {
    const path = resolve(arg);
    return existsSync(path) ? path : null;
  }
  const path = store.pathFor(arg);
  return existsSync(path) ? path : null;
}

function openOrCreate(store: DrawingStore, arg: string): OpenDrawing {
  const existing = findDrawing(store, arg);
  if (existing) {
    const { file, layer } = store.loadSync(existing);
    return { path: existing, name: file.name, layer, createdAt: file.createdAt };
  }
  const path = isPathLike(arg) ? resolve(arg) : store.pathFor(arg);
  const name = isPathLike(arg) ? basename(arg).replace(FILE_EXT, "").replace(/\.json$/, "") : arg;
  const createdAt = new Date().toISOString();
  store.save(path, name, new Layer(), createdAt);
  return { path, name, layer: new Layer(), createdAt };
}

function exportCommand(store: DrawingStore, args: string[]) {
  const { values, positionals } = parseArgs({
    args,
    allowPositionals: true,
    options: {
      basic: { type: "boolean" },
      comment: { type: "string" },
      fenced: { type: "boolean" },
      output: { type: "string", short: "o" },
    },
  });
  const target = positionals[0];
  if (!target) fail("export needs a drawing name or path");
  const path = findDrawing(store, target);
  if (!path) fail(`no drawing named "${target}"`);
  const comment = values.comment ?? "none";
  const wrapper = COMMENT_ALIASES[comment] ?? comment;
  if (!isWrapper(wrapper)) fail(`unknown comment style "${comment}"`);
  const config: ExportConfig = { characters: values.basic ? "basic" : "extended", wrapper, fenced: !!values.fenced };
  const text = `${exportText(store.loadSync(path).layer, config)}\n`;
  if (values.output) writeFileSync(values.output, text);
  else process.stdout.write(text);
}

function systemClipboard(renderer: CliRenderer): Clipboard {
  let internal: string | null = null;
  const local = !process.env.SSH_TTY && !process.env.SSH_CONNECTION;
  const tryRun = (cmd: string[], input?: string): string | null => {
    if (!Bun.which(cmd[0]!)) return null;
    try {
      const r = Bun.spawnSync(cmd, { stdin: input === undefined ? "ignore" : new TextEncoder().encode(input) });
      return r.exitCode === 0 ? r.stdout.toString() : null;
    } catch {
      return null;
    }
  };
  return {
    copy(text) {
      internal = text;
      renderer.copyToClipboardOSC52(text);
      if (local && process.platform === "darwin") tryRun(["pbcopy"], text);
    },
    paste() {
      if (local) {
        const text =
          process.platform === "darwin"
            ? tryRun(["pbpaste"])
            : (tryRun(["wl-paste", "--no-newline"]) ?? tryRun(["xclip", "-selection", "clipboard", "-o"]));
        if (text) return text;
      }
      return internal;
    },
  };
}

async function main() {
  const argv = process.argv.slice(2);
  const store = new DrawingStore();

  if (argv[0] === "export") return exportCommand(store, argv.slice(1));
  if (argv[0] === "list") {
    for (const d of store.list()) console.log(`${d.name}\t${d.size} cells\t${d.path}`);
    return;
  }

  const { values, positionals } = parseArgs({
    args: argv,
    allowPositionals: true,
    options: {
      import: { type: "string" },
      version: { type: "boolean", short: "v" },
      help: { type: "boolean", short: "h" },
    },
  });
  if (values.help) return console.log(HELP);
  if (values.version) return console.log(pkg.version);
  if (!process.stdin.isTTY || !process.stdout.isTTY) fail("needs an interactive terminal (try `lazydraw export`)");

  const config = loadConfig();
  let drawing: OpenDrawing;
  if (values.import) {
    let text: string;
    try {
      text = readFileSync(values.import, "utf8");
    } catch {
      fail(`can't read ${values.import}`);
    }
    const name = store.uniqueName(positionals[0] ?? basename(values.import).replace(/\.[^.]+$/, ""));
    const layer = textToLayer(text);
    const path = store.pathFor(name);
    const createdAt = new Date().toISOString();
    store.save(path, name, layer, createdAt);
    drawing = { path, name, layer, createdAt };
  } else if (positionals[0]) {
    drawing = openOrCreate(store, positionals[0]);
  } else if (config.lastDrawing && existsSync(config.lastDrawing)) {
    drawing = openOrCreate(store, config.lastDrawing);
  } else {
    drawing = openOrCreate(store, store.list()[0]?.name ?? "untitled");
  }
  config.lastDrawing = drawing.path;
  saveConfig(config);

  const renderer = await createCliRenderer({
    useMouse: true,
    enableMouseMovement: true,
    autoFocus: true,
    screenMode: "alternate-screen",
    exitOnCtrlC: false,
    exitSignals: [],
    useKittyKeyboard: {},
  });

  // The `terminal` theme inherits the terminal's own colors, so ask for them before the first
  // frame; painting first and repainting on the answer shows a flash of the stand-in scheme. A
  // terminal too slow to answer in time still gets its colors from the app, a frame or two later.
  const term = config.theme === "terminal" ? await detectTerminalColorsSoon(renderer, PALETTE_WAIT_MS) : null;

  let app: App | null = null;
  let exiting = false;
  const exit = (code: number, error?: unknown) => {
    if (exiting) return;
    exiting = true;
    try {
      app?.shutdown();
    } catch {
      // Still restore the terminal below.
    }
    renderer.destroy();
    if (error) console.error(error);
    process.exit(code);
  };
  process.on("uncaughtException", (e) => exit(1, e));
  process.on("unhandledRejection", (e) => exit(1, e));
  for (const sig of ["SIGTERM", "SIGHUP", "SIGINT"] as const) process.on(sig, () => exit(0));

  app = new App({
    renderer,
    store,
    config,
    drawing,
    clipboard: systemClipboard(renderer),
    term,
    onQuit: () => exit(0),
  });
  app.requestRender();
}

await main();

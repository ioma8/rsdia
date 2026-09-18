#!/usr/bin/env bun
// Captures golden fixtures by replaying test/scenarios/asciiflow.json through
// ASCIIFlow's own drawing code (the fidelity oracle, §9).
//
//   git clone https://github.com/lewish/asciiflow /tmp/asciiflow
//   (cd /tmp/asciiflow && bun install)
//   bun scripts/capture-fixtures.ts /tmp/asciiflow
//
// Writes test/fixtures/asciiflow/<id>.txt and <id>.basic.txt.
import { join, resolve } from "node:path";

const asciiflowDir = process.argv[2];
if (!asciiflowDir) {
  console.error("usage: bun scripts/capture-fixtures.ts <asciiflow checkout>");
  process.exit(2);
}

const root = resolve(import.meta.dir, "..");
const scenarios = resolve(root, "test/scenarios/asciiflow.json");
const outDir = resolve(root, "test/fixtures/asciiflow");

// The runner must live inside the checkout so `#asciiflow/*` imports resolve.
const runner = `
import "#asciiflow/testing/test_setup";
import { store, ToolMode } from "#asciiflow/client/store/index";
import { layerToText } from "#asciiflow/client/text_utils";
import { ASCII, UNICODE } from "#asciiflow/client/constants";
import { Vector } from "#asciiflow/client/vector";
import { Layer } from "#asciiflow/client/layer";

const MODES = { box: ToolMode.BOX, select: ToolMode.SELECT,
  arrow: ToolMode.ARROWS, line: ToolMode.LINES, text: ToolMode.TEXT };
const basic = new Map(Object.keys(UNICODE).map((k) => [UNICODE[k], ASCII[k]]));
const v = ([x, y]) => new Vector(x, y);
const scenarios = await Bun.file(${JSON.stringify(scenarios)}).json();

for (const s of scenarios) {
  localStorage.clear();
  store.setToolMode(ToolMode.BOX);
  store.currentCanvas.committed = new Layer();
  for (const step of s.steps) {
    if (step.tool) store.setToolMode(MODES[step.tool]);
    if (step.drag) {
      const mods = { shift: !!step.shift };
      const [first, ...rest] = step.drag;
      store.currentTool.start(v(first), mods);
      for (const p of rest) store.currentTool.move(v(p), mods);
      store.currentTool.end();
    }
    if (step.text) {
      store.currentTool.start(v(step.text), {});
      for (const ch of step.type) store.currentTool.handleKey(ch, {});
      store.currentTool.handleKey("<enter>", {});
    }
    if (step.key) store.currentTool.handleKey(step.key, {});
  }
  const lines = layerToText(store.currentCanvas.committed).split("\\n").map((l) => l.replace(/ +$/, ""));
  const text = lines.join("\\n") + "\\n";
  await Bun.write(${JSON.stringify(outDir)} + "/" + s.id + ".txt", text);
  await Bun.write(${JSON.stringify(outDir)} + "/" + s.id + ".basic.txt", [...text].map((c) => basic.get(c) ?? c).join(""));
  console.log(s.id, "-", s.title);
}
`;

const runnerPath = join(resolve(asciiflowDir), ".lazydraw-capture.ts");
await Bun.write(runnerPath, runner);
try {
  const proc = Bun.spawn(["bun", runnerPath], { cwd: resolve(asciiflowDir), stdout: "inherit", stderr: "inherit" });
  process.exitCode = await proc.exited;
} finally {
  await Bun.file(runnerPath).delete();
}

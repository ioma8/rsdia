// Replays scenario scripts (test/scenarios/*.json) against the headless editor.
import { Editor, type ToolId } from "../src/core/editor";
import { mods } from "../src/core/tools/tool";
import { Vector } from "../src/core/vector";

export type Point = [number, number];

export interface Step {
  tool?: ToolId;
  drag?: Point[];
  /** Flips the arrow/line elbow, matching ASCIIFlow's Shift-drag in the fixtures it was captured from. */
  flip?: boolean;
  text?: Point;
  type?: string;
  key?: string;
}

export interface Scenario {
  id: string;
  title: string;
  steps: Step[];
}

const v = ([x, y]: Point) => new Vector(x, y);

export function runScenario(steps: Step[], editor = new Editor()): Editor {
  for (const step of steps) {
    if (step.tool) editor.setTool(step.tool);
    if (step.drag) {
      const m = mods({ flip: !!step.flip });
      const [first, ...rest] = step.drag;
      editor.down(v(first!), m);
      for (const p of rest) editor.move(v(p), m);
      editor.up();
    }
    if (step.text) {
      editor.down(v(step.text), mods());
      editor.up();
      for (const ch of step.type ?? "") editor.key(ch, mods());
      editor.text.commit();
    }
    if (step.key) editor.key(step.key, mods());
  }
  return editor;
}

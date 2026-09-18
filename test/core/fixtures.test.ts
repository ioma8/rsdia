// Golden fixtures captured from ASCIIFlow itself (scripts/capture-fixtures.ts).
import { describe, expect, test } from "bun:test";
import { exportText } from "../../src/core/export";
import scenarios from "../scenarios/asciiflow.json";
import { runScenario, type Scenario } from "../scenario";

const dir = `${import.meta.dir}/../fixtures/asciiflow`;

describe("ASCIIFlow fixtures", () => {
  for (const s of scenarios as Scenario[]) {
    test(`${s.id} ${s.title}`, async () => {
      const editor = runScenario(s.steps);
      const committed = editor.canvas.committed;
      const extended = exportText(committed, { characters: "extended", wrapper: "none", fenced: false });
      const basic = exportText(committed, { characters: "basic", wrapper: "none", fenced: false });
      expect(`${extended}\n`).toBe(await Bun.file(`${dir}/${s.id}.txt`).text());
      expect(`${basic}\n`).toBe(await Bun.file(`${dir}/${s.id}.basic.txt`).text());
    });
  }
});

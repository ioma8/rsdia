import { describe, expect, test } from "bun:test";
import { applyExportConfig, exportText, WRAPPERS, type Wrapper } from "../../src/core/export";
import { textToLayer } from "../../src/core/text";

const drawing = textToLayer(["┌─┐  ", "│ ├─►", "└─┘"].join("\n"), undefined);

// One golden per comment style (M9).
const GOLDEN: Record<Wrapper, string> = {
  none: "┌─┐\n│ ├─►\n└─┘",
  star: "/*\n┌─┐\n│ ├─►\n└─┘\n */",
  "star-filled": "/*\n * ┌─┐\n * │ ├─►\n * └─┘\n */",
  "triple-quotes": 'u"""\n┌─┐\n│ ├─►\n└─┘\n"""',
  hash: "# ┌─┐\n# │ ├─►\n# └─┘",
  slash: "// ┌─┐\n// │ ├─►\n// └─┘",
  "three-slashes": "/// ┌─┐\n/// │ ├─►\n/// └─┘",
  dash: "-- ┌─┐\n-- │ ├─►\n-- └─┘",
  apostrophe: "' ┌─┐\n' │ ├─►\n' └─┘",
  backticks: "```\n┌─┐\n│ ├─►\n└─┘\n```",
  "four-spaces": "    ┌─┐\n    │ ├─►\n    └─┘",
  semicolon: "; ┌─┐\n; │ ├─►\n; └─┘",
};

describe("export", () => {
  for (const { id } of WRAPPERS) {
    test(`wrapper ${id}`, () => {
      expect(exportText(drawing, { characters: "extended", wrapper: id, fenced: false })).toBe(GOLDEN[id]);
    });
  }

  test("basic charset", () => {
    expect(exportText(drawing, { characters: "basic", wrapper: "none", fenced: false })).toBe("+-+\n| +->\n+-+");
  });

  test("basic triple quotes drop the u prefix", () => {
    expect(applyExportConfig("x", { characters: "basic", wrapper: "triple-quotes", fenced: false })).toBe(
      '"""\nx\n"""',
    );
  });

  test("markdown fence wraps everything", () => {
    expect(exportText(drawing, { characters: "basic", wrapper: "hash", fenced: true })).toBe(
      "```text\n# +-+\n# | +->\n# +-+\n```",
    );
  });

  test("bounding box excludes empty margins and trims trailing spaces", () => {
    const l = textToLayer("   \n   a   \n\n     b");
    expect(exportText(l, { characters: "extended", wrapper: "none", fenced: false })).toBe("a\n\n  b");
  });

  test("empty drawing exports as empty text", () => {
    expect(exportText(textToLayer(""), { characters: "extended", wrapper: "none", fenced: false })).toBe("");
  });
});

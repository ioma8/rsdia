// Export formatting. Wrapper semantics ported from ASCIIFlow (client/export.tsx), MIT © Lewis Hemens.
import { BASIC } from "./glyphs";
import type { LayerView } from "./layer";
import { layerToText } from "./text";

export const WRAPPERS = [
  { id: "none", label: "none" },
  { id: "star", label: "/* */" },
  { id: "star-filled", label: "/***/" },
  { id: "triple-quotes", label: '""" """' },
  { id: "hash", label: "# hash" },
  { id: "slash", label: "// slash" },
  { id: "three-slashes", label: "/// triple" },
  { id: "dash", label: "-- dash" },
  { id: "apostrophe", label: "' apostrophe" },
  { id: "backticks", label: "``` backticks" },
  { id: "four-spaces", label: "    indent" },
  { id: "semicolon", label: "; semicolon" },
] as const;

export type Wrapper = (typeof WRAPPERS)[number]["id"];
export type Charset = "extended" | "basic";

export interface ExportConfig {
  characters: Charset;
  wrapper: Wrapper;
  /** Wrap the result in a Markdown code fence. */
  fenced: boolean;
}

export const DEFAULT_EXPORT: ExportConfig = { characters: "extended", wrapper: "none", fenced: false };

export function isWrapper(value: string): value is Wrapper {
  return WRAPPERS.some((w) => w.id === value);
}

export function toBasic(text: string): string {
  return [...text].map((ch) => BASIC.get(ch) ?? ch).join("");
}

export function applyExportConfig(text: string, config: ExportConfig): string {
  if (config.characters === "basic") text = toBasic(text);
  let lines = text.split("\n");
  const prefix = (p: string) => (lines = lines.map((l) => `${p}${l}`));
  switch (config.wrapper) {
    case "star":
      lines = ["/*", ...lines, " */"];
      break;
    case "star-filled":
      lines = ["/*", ...lines.map((l) => ` * ${l}`), " */"];
      break;
    case "triple-quotes":
      lines = [config.characters === "basic" ? '"""' : 'u"""', ...lines, '"""'];
      break;
    case "hash":
      prefix("# ");
      break;
    case "slash":
      prefix("// ");
      break;
    case "three-slashes":
      prefix("/// ");
      break;
    case "dash":
      prefix("-- ");
      break;
    case "apostrophe":
      prefix("' ");
      break;
    case "backticks":
      lines = ["```", ...lines, "```"];
      break;
    case "four-spaces":
      prefix("    ");
      break;
    case "semicolon":
      prefix("; ");
      break;
  }
  if (config.fenced) lines = ["```text", ...lines, "```"];
  return lines.join("\n");
}

/** Drawing text: bounding box of all non-empty cells, trailing spaces trimmed per row. */
export function exportText(layer: LayerView, config: ExportConfig): string {
  return applyExportConfig(layerToText(layer, null, true), config);
}

// What popovers and dialogs need from the app.
import type { Editor } from "../core/editor";
import type { ExportConfig } from "../core/export";
import type { Config, GridStyle, ThemeChoice } from "../storage/config";
import type { DrawingInfo } from "../storage/drawings";
import type { ItemId, PanelId } from "./toolbar";

export interface InputDialog {
  kind: "input";
  title: string;
  label: string;
  value: string;
  cursor: number;
  error: string | null;
  /** Returns an error message to keep the dialog open, or null to close it. */
  submit(value: string): string | null | Promise<string | null>;
}

export interface ConfirmDialog {
  kind: "confirm";
  title: string;
  message: string;
  yes: string;
  confirm(): void;
}

export type Dialog = InputDialog | ConfirmDialog;

export interface Host {
  readonly editor: Editor;
  readonly config: Config;
  readonly drawingName: string;
  readonly drawings: DrawingInfo[];
  readonly currentPath: string;
  panel: PanelId | null;

  closePanel(): void;
  activate(id: ItemId, anchorX: number): void;
  openDialog(d: Dialog): void;
  closeDialog(): void;
  toast(message: string): void;
  requestRender(): void;

  // files
  openDrawing(path: string): void;
  newDrawing(): void;
  renameDrawing(): void;
  forkDrawing(): void;
  deleteDrawing(): void;
  clearDrawing(): void;
  importText(): void;

  // export
  setExport(patch: Partial<ExportConfig>): void;
  exportPreview(): string;
  copyExport(): void;
  saveExport(): void;

  // settings
  setGrid(g: GridStyle): void;
  setTheme(t: ThemeChoice): void;
  setCopyOnSelect(on: boolean): void;
  recenter(): void;
}

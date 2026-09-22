#!/usr/bin/env bun
// Opens the lazydraw pane in the running herdr session.
//
// herdr keybindings can only invoke plugin *actions*, and there is no CLI
// wrapper for arbitrary socket methods, so this action speaks the raw
// newline-delimited JSON protocol on HERDR_SOCKET_PATH: one request per line,
// one response line back carrying the same id.
import { connect } from "node:net";

const socketPath = process.env.HERDR_SOCKET_PATH;
const pluginId = process.env.HERDR_PLUGIN_ID ?? "mpospirit.lazydraw";

function fail(message: string): never {
  console.error(`lazydraw: ${message}`);
  process.exit(1);
}

if (!socketPath) fail("HERDR_SOCKET_PATH is not set — run this through herdr");

const request = {
  id: `lazydraw-${process.pid}`,
  method: "plugin.pane.open",
  params: {
    plugin_id: pluginId,
    entrypoint: "canvas",
    placement: "split",
    direction: "right",
    focus: true,
  },
};

const socket = connect(socketPath);
let buffer = "";

socket.on("connect", () => socket.write(`${JSON.stringify(request)}\n`));

socket.on("data", (chunk) => {
  buffer += chunk.toString();
  const newline = buffer.indexOf("\n");
  if (newline === -1) return;

  socket.end();
  let response: { error?: { message?: string } };
  try {
    response = JSON.parse(buffer.slice(0, newline));
  } catch {
    fail(`unreadable response from herdr: ${buffer.slice(0, newline)}`);
  }
  if (response.error) fail(response.error.message ?? "herdr refused to open the pane");
  process.exit(0);
});

socket.on("error", (error) => fail(`cannot reach herdr: ${error.message}`));

/**
 * EvAgent OpenTUI bootstrap entry.
 *
 * Per spec/02-terminal-ui.md:
 *   1. Import and call ensureSolidTransformPlugin() from @opentui/solid/bun-plugin
 *   2. Create a CliRenderer with createCliRenderer({ targetFps, exitOnCtrlC, useMouse })
 *   3. Create default keymap
 *   4. render(() => <App />, renderer)  ← renderer MUST be passed explicitly
 */

import { ensureSolidTransformPlugin } from "@opentui/solid/bun-plugin";

// Step 1: install the SolidJS JSX transform plugin BEFORE any .tsx import.
ensureSolidTransformPlugin();

// Step 2-4: create renderer + render App.
const { createCliRenderer, createDefaultOpenTuiKeymap } = await import("@opentui/solid");
const { render } = await import("@opentui/solid");
const { createCliRenderer: _ccr } = await import("@opentui/solid");
void _ccr;

// Defer importing the .tsx until after the plugin is registered.
const App = (await import("./src/index.tsx")).default;

const renderer = createCliRenderer({
  targetFps: 60,
  exitOnCtrlC: true,
  useMouse: true,
});

createDefaultOpenTuiKeymap(renderer);

render(() => <App />, renderer);

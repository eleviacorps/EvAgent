// EvAgent OpenTUI entry point
// This registers the SolidJS transform plugin and starts the app

import { ensureSolidTransformPlugin } from "@opentui/solid/bun-plugin"

// Register the SolidJS JSX transform plugin for bun
ensureSolidTransformPlugin()

// Now import and run the app
await import("./index")

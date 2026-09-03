// Runs a WASI Preview 1 command with the repository root preopened as fd 3.
import { readFile } from "node:fs/promises";
import { WASI } from "node:wasi";

const modulePath = process.argv[2];
const preopenPath = process.argv[3] ?? process.cwd();
if (!modulePath) {
  console.error("usage: node tools/run_wasi_smoke.mjs <module.wasm> [preopen-dir]");
  process.exit(2);
}

const wasi = new WASI({
  version: "preview1",
  args: [],
  env: process.env,
  preopens: { ".": preopenPath },
});
const module = await WebAssembly.compile(await readFile(modulePath));
const instance = await WebAssembly.instantiate(module, wasi.getImportObject());
const leaked = Object.keys(instance.exports).filter((name) => name.startsWith("__wave_"));
if (leaked.length !== 0) {
  throw new Error(`private Wave functions leaked into exports: ${leaked.join(", ")}`);
}
wasi.start(instance);

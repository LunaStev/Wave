// Instantiates the compiler's minimal browser-hosted WebAssembly fixture.
import { readFile } from "node:fs/promises";

const modulePath = process.argv[2];
if (!modulePath) {
  console.error("usage: node tools/run_wasm_smoke.mjs <module.wasm>");
  process.exit(2);
}

const bytes = await readFile(modulePath);
const imports = {
  env: {
    host_add(a, b) {
      return a + b;
    },
  },
};
const { instance } = await WebAssembly.instantiate(bytes, imports);
const {
  wave_add: add,
  wave_features: exerciseFeatures,
  main,
  memory,
} = instance.exports;
const leaked = Object.keys(instance.exports).filter((name) => name.startsWith("__wave_"));
if (leaked.length !== 0 || "add" in instance.exports) {
  throw new Error(`private Wave functions leaked into exports: ${leaked.join(", ")}`);
}

if (typeof add !== "function" || add(7, 5) !== 12) {
  throw new Error("exported add function returned an unexpected value");
}
if (typeof exerciseFeatures !== "function" || exerciseFeatures(5) !== 20) {
  throw new Error("WebAssembly language feature smoke test failed");
}
if (typeof main !== "function" || main() !== 42) {
  throw new Error("exported main function returned an unexpected value");
}
if (!(memory instanceof WebAssembly.Memory)) {
  throw new Error("module did not export linear memory");
}

console.log("WebAssembly smoke test passed");

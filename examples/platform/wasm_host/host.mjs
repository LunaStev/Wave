import { readFile } from "node:fs/promises";
import assert from "node:assert/strict";

const bytes = await readFile(process.argv[2]);
const { instance } = await WebAssembly.instantiate(bytes, {
    env: { host_add: (left, right) => left + right }
});
assert.equal(instance.exports.wave_answer(40), 42);
console.log("Wave host interoperability passed");

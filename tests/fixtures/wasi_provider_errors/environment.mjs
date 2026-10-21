// SPDX-License-Identifier: MPL-2.0
// Construct WASI's environment independently of host process startup settings
// and host limits on environment-variable lengths (notably on Windows).
import { readFileSync } from "node:fs";
import { dirname } from "node:path";
import { WASI } from "node:wasi";

const [modulePath, mode, key, length] = process.argv.slice(2);
const env = {};
if (mode === "growth") {
    const filler = "x".repeat(Number(length));
    if (key === "AAA") {
        env.AAA = filler;
        env.PROBE = "ok";
    } else if (key === "ZZZ") {
        env.PROBE = "ok";
        env.ZZZ = filler;
    } else {
        throw new Error(`unexpected environment key: ${key}`);
    }
} else if (mode === "entry-limit") {
    for (let n = 0; n < 1025; n++) {
        env[`KEY${n}`] = "v";
    }
} else {
    throw new Error(`unexpected environment test mode: ${mode}`);
}
const wasi = new WASI({
    version: "preview1", returnOnExit: true, args: [], env,
    preopens: { ".": dirname(modulePath) },
});
const module = new WebAssembly.Module(readFileSync(modulePath));
const instance = new WebAssembly.Instance(module, wasi.getImportObject());
process.exit(wasi.start(instance));

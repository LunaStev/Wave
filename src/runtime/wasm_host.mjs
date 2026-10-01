// SPDX-License-Identifier: MPL-2.0
// Embedded in wavec: the Node execution host for Wave WebAssembly modules.
import { readFile } from "node:fs/promises";
import { writeSync } from "node:fs";
import { WASI } from "node:wasi";

function stdout(bytes) {
  let offset = 0;
  while (offset < bytes.length) {
    const written = writeSync(1, bytes, offset, bytes.length - offset);
    if (written === 0) throw new Error("WebAssembly stdout made no progress");
    offset += written;
  }
}

// printf's default %f precision, rounded from the exact binary value with
// ties to even. Number.toFixed loses fixed notation at 1e21 and rounds ties
// differently; integer arithmetic also preserves negative zero and subnormals.
export function fixedDouble(value) {
  const bits = new DataView(new ArrayBuffer(8));
  bits.setFloat64(0, value, true);
  const raw = bits.getBigUint64(0, true);
  const sign = raw >> 63n ? "-" : "";
  const exponent = Number((raw >> 52n) & 2047n);
  const fraction = raw & ((1n << 52n) - 1n);
  if (exponent === 2047) return sign + (fraction ? "nan" : "inf");
  let scaled = (exponent ? fraction | (1n << 52n) : fraction) * 1000000n;
  const shift = exponent ? exponent - 1023 - 52 : -1074;
  if (shift >= 0) {
    scaled <<= BigInt(shift);
  } else {
    const divisor = 1n << BigInt(-shift);
    const quotient = scaled / divisor;
    const twiceRemainder = (scaled % divisor) * 2n;
    scaled = quotient + BigInt(twiceRemainder > divisor ||
      (twiceRemainder === divisor && (quotient & 1n) !== 0n));
  }
  return sign + (scaled / 1000000n) + "." +
    (scaled % 1000000n).toString().padStart(6, "0");
}

// These are the output formats emitted by Wave's typed I/O lowering. This is
// not a general libc: unsupported external printf formats fail explicitly.
export function createOutputHost(getMemory, memory64, write = stdout) {
  const pointerWidth = memory64 ? 8 : 4;
  function memory() {
    const result = getMemory();
    if (!(result instanceof WebAssembly.Memory)) {
      throw new Error("WebAssembly output requires exported linear memory");
    }
    return new Uint8Array(result.buffer);
  }
  function address(pointer) {
    const value = memory64 ? BigInt.asUintN(64, BigInt(pointer)) : BigInt(Number(pointer) >>> 0);
    if (value > BigInt(Number.MAX_SAFE_INTEGER)) throw new RangeError("WebAssembly pointer exceeds host address range");
    return Number(value);
  }
  function string(pointer) {
    const bytes = memory();
    const start = address(pointer);
    if (start >= bytes.length) throw new RangeError("WebAssembly string is outside linear memory");
    const end = bytes.indexOf(0, start);
    if (end < 0) throw new RangeError("WebAssembly string has no NUL terminator");
    return bytes.subarray(start, end);
  }
  return {
    puts(pointer) {
      const bytes = Buffer.concat([string(pointer), Buffer.from("\n")]);
      write(bytes);
      return bytes.length;
    },
    printf(format, argumentsPointer) {
      const formatBytes = string(format);
      let cursor = address(argumentsPointer);
      const chunks = [];
      function argument(size, kind) {
        cursor = Math.ceil(cursor / size) * size;
        const buffer = memory().buffer;
        if (cursor + size > buffer.byteLength) throw new RangeError("WebAssembly printf argument is outside linear memory");
        const view = new DataView(buffer);
        const value = kind === "float" ? view.getFloat64(cursor, true) :
          size === 8 ? view.getBigUint64(cursor, true) : view.getUint32(cursor, true);
        cursor += size;
        return value;
      }
      let start = 0;
      for (let i = 0; i < formatBytes.length; i++) {
        if (formatBytes[i] !== 37) continue;
        chunks.push(formatBytes.subarray(start, i));
        const spec = formatBytes[++i];
        switch (spec) {
          case 37: chunks.push(Buffer.from("%")); break;
          case 115: chunks.push(string(argument(pointerWidth))); break;
          case 99: chunks.push(Buffer.from([Number(argument(4)) & 255])); break;
          case 102: chunks.push(Buffer.from(fixedDouble(argument(8, "float")))); break;
          case 112: {
            const pointer = argument(pointerWidth);
            chunks.push(Buffer.from(BigInt(pointer) === 0n ? "(nil)" : "0x" + pointer.toString(16)));
            break;
          }
          default: throw new Error("unsupported WebAssembly printf format at byte " + (i - 1));
        }
        start = i + 1;
      }
      chunks.push(formatBytes.subarray(start));
      const bytes = Buffer.concat(chunks);
      write(bytes);
      return bytes.length;
    },
  };
}

export async function runWaveModule({ memory64, wasi: useWasi }) {
  const modulePath = process.argv[1];
  let instance;
  const output = createOutputHost(() => instance?.exports.memory, memory64);
  let imports = { env: output };
  let wasi;
  if (useWasi) {
    wasi = new WASI({ version: "preview1", args: process.argv.slice(1),
      env: process.env, preopens: { ".": process.cwd() }, returnOnExit: true });
    imports = wasi.getImportObject();
    // Explicit extern(c) functions use the target's import namespace, while
    // compiler-generated printf uses env. Preserve actual WASI API functions.
    Object.assign(imports.wasi_snapshot_preview1, output);
    imports.env = output;
  }
  const module = await WebAssembly.compile(await readFile(modulePath));
  instance = await WebAssembly.instantiate(module, imports);
  if (wasi) {
    process.exitCode = wasi.start(instance);
  } else {
    if (typeof instance.exports.main !== "function") throw new Error("WebAssembly module does not export main");
    const status = memory64 ? instance.exports.main(0, 0n) : instance.exports.main();
    if (Number.isInteger(status) && status !== 0) process.exitCode = status;
  }
}

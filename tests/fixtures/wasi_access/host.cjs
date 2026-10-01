// SPDX-License-Identifier: MPL-2.0
const assert = require('node:assert/strict');
const fs = require('node:fs');
const { WASI } = require('node:wasi');
(async () => {
  const module = await WebAssembly.compile(fs.readFileSync(process.argv[2]));
  let instance, calls, options;
  const path = (pointer, length) => Buffer.from(instance.exports.memory.buffer, pointer, length).toString();
  const imports = {
    path_filestat_get(fd, flags, pointer, length, result) {
      calls.push('stat');
      assert.equal(fd, 3); assert.equal(flags, 0); assert.equal(path(pointer, length), 'fixture');
      if (options.statError) return options.statError;
      new DataView(instance.exports.memory.buffer).setUint8(result + 16, options.type ?? 4);
      return 0;
    },
    path_open(fd, flags, pointer, length, oflags, rights, inherited, fdflags, result) {
      calls.push('open');
      assert.equal(fd, 3); assert.equal(flags, 0); assert.equal(path(pointer, length), 'fixture');
      assert.equal(oflags, 0); assert.equal(inherited, 0n); assert.equal(fdflags, 0);
      assert.equal(rights, options.rights);
      if (options.openError) return options.openError;
      new DataView(instance.exports.memory.buffer).setUint32(result, 9, true);
      return 0;
    },
    fd_close(fd) { calls.push('close'); assert.equal(fd, 9); return options.closeError ?? 0; },
  };
  instance = await WebAssembly.instantiate(module, { wasi_snapshot_preview1: imports });
  const run = (mode, expected, expectedCalls, config = {}) => {
    calls = []; options = config;
    assert.equal(instance.exports.check_access(mode), BigInt(expected), `mode ${mode}: ${JSON.stringify(config, (_, v) => typeof v === 'bigint' ? String(v) : v)}`);
    assert.deepEqual(calls, expectedCalls);
  };
  run(0, 0, ['stat']);
  for (const [mode, rights] of [[4, 2n], [2, 64n], [6, 66n]]) {
    run(mode, 0, ['stat', 'open', 'close'], { rights });
    for (const openError of [2, 63, 76]) run(mode, -openError, ['stat', 'open'], { rights, openError });
    run(mode, -8, ['stat', 'open', 'close'], { rights, closeError: 8 });
    run(mode, -58, ['stat'], { type: 3 });
    run(mode, -58, ['stat'], { type: 7 });
  }
  for (const mode of [1, 3, 5, 7]) run(mode, -58, []);
  for (const mode of [8, 9, -1]) run(mode, -28, []);
  for (const mode of [0, 2, 4, 6]) {
    run(mode, -44, ['stat'], { statError: 44 });
    run(mode, -76, ['stat'], { statError: 76 });
  }
  run(0, 0, ['stat'], { type: 3 });

  // The real WASI host confirms both existence and non-destructive open/close.
  const wasi = new WASI({ version: 'preview1', preopens: { '.': process.cwd() }, returnOnExit: true });
  const real = await WebAssembly.instantiate(module, wasi.getImportObject());
  wasi.initialize(real);
  const bytes = Buffer.from([0, 1, 255, 42, 10]);
  fs.writeFileSync('fixture', bytes);
  for (const mode of [0, 4, 2, 6]) {
    assert.equal(real.exports.check_access(mode), 0n);
    assert.deepEqual(fs.readFileSync('fixture'), bytes);
  }
  // Revoke only PATH_OPEN from the real preopen. Stat must still succeed,
  // while R/W checks preserve the host's capability failure (ENOTCAPABLE).
  const wasiImports = wasi.getImportObject().wasi_snapshot_preview1;
  const scratch = real.exports.memory.buffer.byteLength - 64;
  assert.equal(wasiImports.fd_fdstat_get(3, scratch), 0);
  const stat = new DataView(real.exports.memory.buffer);
  const rights = stat.getBigUint64(scratch + 8, true);
  const inherited = stat.getBigUint64(scratch + 16, true);
  assert.equal(wasiImports.fd_fdstat_set_rights(3, rights & ~(1n << 13n), inherited), 0);
  assert.equal(real.exports.check_access(0), 0n);
  for (const mode of [4, 2, 6]) assert.equal(real.exports.check_access(mode), -76n);
  assert.deepEqual(fs.readFileSync('fixture'), bytes);
  fs.unlinkSync('fixture');
  assert.equal(real.exports.check_access(0), -44n);
  assert.equal(real.exports.check_access(4), -44n);
  console.log('WASI access contracts passed');
})().catch(error => { console.error(error); process.exitCode = 1; });

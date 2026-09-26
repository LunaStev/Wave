// SPDX-License-Identifier: MPL-2.0
// Use Node's real WASI host in a child: synchronous poll_oneoff can block the
// event loop, so an in-process timer cannot enforce this regression's deadline.
const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const fs = require('node:fs');

if (process.argv[2] !== '--child') {
    const result = spawnSync(process.execPath, ['--no-warnings', __filename, '--child', process.argv[2]], {
        encoding: 'utf8', timeout: 10000, killSignal: 'SIGKILL',
    });
    assert.ifError(result.error);
    assert.equal(result.status, 0, `real WASI sleep failed (${result.signal ?? result.status})\n${result.stdout}${result.stderr}`);
    process.stdout.write(result.stdout);
} else {
    (async () => {
        const { WASI } = require('node:wasi');
        const wasi = new WASI({ version: 'preview1', args: [], env: {}, preopens: {} });
        const module = await WebAssembly.compile(fs.readFileSync(process.argv[3]));
        const instance = await WebAssembly.instantiate(module, wasi.getImportObject());
        wasi.initialize(instance);
        assert.equal(instance.exports.sleep_ns(0n), 0n);
        assert.equal(instance.exports.sleep_ns(-1n), -28n);
        // Exercise real host completion; exact retry durations are checked by
        // host.cjs without depending on the host's timer rounding or scheduling.
        for (const [name, duration] of [['sleep_ns', 1000000n], ['sleep_ns', 3000000n], ['raw_sleep_ns', 2000000n]]) {
            assert.equal(instance.exports[name](duration), 0n);
        }
        console.log('Real Node WASI sleep passed');
    })().catch(error => { console.error(error); process.exitCode = 1; });
}

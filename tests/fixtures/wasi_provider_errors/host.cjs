// SPDX-License-Identifier: MPL-2.0
const assert = require('node:assert/strict');
const fs = require('node:fs');

(async () => {
    const module = await WebAssembly.compile(fs.readFileSync(process.argv[2]));
    let instance;
    let mode = 'success';
    let hostCalls = 0;
    let ioCalls = 0;
    const memory = () => new DataView(instance.exports.memory.buffer);
    const error = () => mode === 'host-error' ? 22 : 0;
    const imports = { wasi_snapshot_preview1: {
        fd_filestat_get(fd, output) {
            hostCalls += 1;
            if (error()) { return error(); }
            const view = memory();
            view.setUint8(output + 16, 4);
            view.setBigUint64(output + 32, mode === 'overflow' ? (1n << 63n) : 42n, true);
            view.setBigUint64(output + 48, 12345678901n, true);
            return 0;
        },
        fd_seek(fd, offset, whence, output) {
            hostCalls += 1;
            if (error()) { return error(); }
            memory().setBigUint64(output, mode === 'overflow' ? (1n << 63n) : 42n, true);
            return 0;
        },
        clock_time_get(id, precision, output) {
            hostCalls += 1;
            if (error()) { return error(); }
            memory().setBigUint64(output, 12345678901n, true);
            return 0;
        },
        environ_sizes_get(count, bytes) {
            hostCalls += 1;
            if (error()) { return error(); }
            memory().setUint32(count, 0, true);
            memory().setUint32(bytes, 0, true);
            return 0;
        },
        environ_get() {
            hostCalls += 1;
            return error();
        },
        fd_read(fd, vectors, count, output) {
            hostCalls += 1;
            ioCalls += 1;
            assert.ok(ioCalls <= 8, 'descriptor retry exceeded its host-call budget');
            if (mode === 'interrupted' && ioCalls < 3) { return 27; }
            if (mode === 'permanent') { return 4; }
            if (error()) { return error(); }
            const view = memory();
            view.setUint8(view.getUint32(vectors, true), 65);
            view.setUint32(output, 1, true);
            return 0;
        },
        fd_write(fd, vectors, count, output) {
            hostCalls += 1;
            ioCalls += 1;
            assert.ok(ioCalls <= 8, 'descriptor retry exceeded its host-call budget');
            if (mode === 'interrupted' && ioCalls < 3) { return 27; }
            if (mode === 'permanent') { return 4; }
            if (error()) { return error(); }
            const view = memory();
            assert.equal(view.getUint8(view.getUint32(vectors, true)), 65);
            view.setUint32(output, 1, true);
            return 0;
        }
    } };
    // Unexpected host calls fail instead of being silently treated as success.
    for (const item of WebAssembly.Module.imports(module)) {
        imports[item.module] ??= {};
        imports[item.module][item.name] ??= () => {
            throw new Error(`Unexpected host import: ${item.module}.${item.name}`);
        };
    }
    instance = await WebAssembly.instantiate(module, imports);
    assert.equal(instance.exports.invalid_arguments(), 0);
    assert.equal(instance.exports.unsupported_calls(), 0);
    assert.equal(hostCalls, 0, 'local failures must not call the host');
    for (const scenario of ['overflow', 'host-error', 'success']) {
        mode = scenario;
        const expected = scenario === 'overflow' ? -61n : scenario === 'host-error' ? -22n : 0n;
        assert.equal(instance.exports.file_stat(), expected, scenario);
        assert.equal(instance.exports.file_seek(), scenario === 'success' ? 42n : expected, scenario);
    }
    for (const scenario of ['host-error', 'success']) {
        mode = scenario;
        assert.equal(instance.exports.clock_value(), scenario === 'host-error' ? -22n : 0n, scenario);
        assert.equal(instance.exports.environment_value(), scenario === 'host-error' ? -22n : 0n, scenario);
    }
    for (const operation of ['read_value', 'write_value']) {
        for (const scenario of ['interrupted', 'permanent', 'host-error', 'success']) {
            mode = scenario;
            ioCalls = 0;
            const expected = scenario === 'permanent' ? -4n : scenario === 'host-error' ? -22n : 1n;
            assert.equal(instance.exports[operation](), expected, `${operation} ${scenario}`);
            assert.equal(ioCalls, scenario === 'interrupted' ? 3 : 1, `${operation} ${scenario}`);
        }
    }
    console.log('WASI provider argument, overflow, host-error and retry checks passed');
})().catch(error => {
    console.error(error);
    process.exit(1);
});

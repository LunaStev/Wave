// SPDX-License-Identifier: MPL-2.0
const fs = require('node:fs');
const assert = require('node:assert/strict');
(async () => {
    const {instance} = await WebAssembly.instantiate(fs.readFileSync(process.argv[2]), {});
    const f = instance.exports;
    assert.equal(f.shift(1n, 63n), -(1n << 63n));
    assert.equal(f.signed_shift(-128n, 1), -64n);
    for (const n of [64n, 256n, 1n << 32n, -1n]) assert.throws(() => f.shift(1n, n), WebAssembly.RuntimeError);
    assert.throws(() => f.signed_shift(1n, -1), WebAssembly.RuntimeError);
    assert.equal(f.signed_cast(-128.9), -128);
    assert.equal(f.signed_cast(127.9), 127);
    assert.equal(f.unsigned_cast(-0.5), 0);
    assert.equal(f.unsigned_cast(255.9), 255);
    for (const x of [NaN, Infinity, -Infinity, -129, 128]) assert.throws(() => f.signed_cast(x), WebAssembly.RuntimeError);
    for (const x of [NaN, Infinity, -Infinity, -1, 256]) assert.throws(() => f.unsigned_cast(x), WebAssembly.RuntimeError);
    for (const x of [0, -0]) assert.equal(f.truth(x), 0);
    for (const x of [2, -0.5, NaN, Infinity, -Infinity]) assert.equal(f.truth(x), 1);
    assert.equal(f.wide(3.75), 0);
    assert.throws(() => f.wide(-1), WebAssembly.RuntimeError);
    console.log('checked numeric boundaries and traps passed');
})().catch(error => { console.error(error); process.exitCode = 1; });

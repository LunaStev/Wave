// SPDX-License-Identifier: MPL-2.0
const assert = require('node:assert/strict');
const fs = require('node:fs');
(async () => {
    const module = await WebAssembly.compile(fs.readFileSync(process.argv[2]));
    assert.deepEqual(WebAssembly.Module.imports(module), [], 'arithmetic must be freestanding');
    assert.ok(!WebAssembly.Module.exports(module).some(x => x.name.startsWith('__wave.runtime.')));
    const { exports: e } = await WebAssembly.instantiate(module, {});
    const mem = new DataView(e.memory.buffer);
    const mask = (1n << 128n) - 1n;
    const put = (offset, value) => {
        value = BigInt.asUintN(128, value);
        mem.setBigUint64(offset, value & ((1n << 64n) - 1n), true);
        mem.setBigUint64(offset + 8, value >> 64n, true);
    };
    const get = offset => mem.getBigUint64(offset, true) | (mem.getBigUint64(offset + 8, true) << 64n);
    // Above the stack and below the module's initial memory bound.
    const a = 70000, b = a + 16, output = b + 16;
    let seed = 42n;
    const random = () => (seed = (seed * 6364136223846793005n + 1442695040888963407n) & mask);
    const values = [0n, 1n, 2n, 3n, (1n << 64n) - 1n, 1n << 64n, (1n << 127n) - 1n, 1n << 127n, mask];
    for (let i = 0; i < 60; i++) values.push(random());
    for (const x of values) for (const y of values) {
        put(a, x); put(b, y);
        e.mul(BigInt(a), BigInt(b), BigInt(output)); assert.equal(get(output), (x * y) & mask);
        if (y) for (const [name, expected] of [['udiv', x / y], ['urem', x % y], ['compound', x / y]]) {
            e[name](BigInt(a), BigInt(b), BigInt(output)); assert.equal(get(output), expected, `${name} ${x} ${y}`);
        }
        const sx = BigInt.asIntN(128, x), sy = BigInt.asIntN(128, y);
        if (sy && !(sx === -(1n << 127n) && sy === -1n)) {
            for (const [name, expected] of [['sdiv', sx / sy], ['srem', sx % sy]]) {
                e[name](BigInt(a), BigInt(b), BigInt(output)); assert.equal(BigInt.asIntN(128, get(output)), expected, `${name} ${sx} ${sy}`);
            }
        }
    }
    for (const x of values) for (const n of [0n, 1n, 31n, 32n, 63n, 64n, 65n, 127n]) {
        put(a, x); put(b, n);
        for (const [name, expected] of [['shl', (x << n) & mask], ['lshr', x >> n], ['ashr', BigInt.asUintN(128, BigInt.asIntN(128, x) >> n)]]) {
            e[name](BigInt(a), BigInt(b), BigInt(output)); assert.equal(get(output), expected, `${name} ${x} ${n}`);
        }
    }
    // IEEE ties-to-even, including exact halfway values and a sticky low bit.
    const casts = [...values];
    for (const p of [24n, 53n]) for (const shift of [1n, 10n, 60n]) {
        for (const low of [-1n, 0n, 1n]) casts.push((1n << (p + shift)) + (1n << shift) + low);
    }
    const integerF32 = x => {
        const negative = x < 0n;
        if (negative) x = -x;
        const shift = Math.max(0, x.toString(2).length - 24);
        if (!shift) return Number(negative ? -x : x);
        const unit = 1n << BigInt(shift), half = unit / 2n;
        let head = x / unit;
        const tail = x % unit;
        if (tail > half || (tail === half && (head & 1n))) head++;
        return Math.fround(Number(negative ? -head : head) * 2 ** shift);
    };
    for (const x of casts) {
        put(a, x);
        assert.equal(e.u_to_f64(BigInt(a)), Number(x));
        assert.equal(e.s_to_f64(BigInt(a)), Number(BigInt.asIntN(128, x)));
        // Round the integer directly; Number -> f32 can double-round halfway samples.
        assert.equal(e.u_to_f32(BigInt(a)), integerF32(x));
        assert.equal(e.s_to_f32(BigInt(a)), integerF32(BigInt.asIntN(128, x)));
    }
    for (const n of [0, -0, 0.75, -0.75, 1, -1, 123.875, -123.875, 2 ** 63, -(2 ** 63), 2 ** 100, -(2 ** 100), -(2 ** 127), 2 ** 127]) {
        for (const [float, value] of [['f64', n], ['f32', Math.fround(n)]]) {
            for (const signed of [true, false]) {
                if ((!signed && value <= -1) || (signed && value >= 2 ** 127)) continue;
                e[`${float}_to_${signed ? 's' : 'u'}`](value, BigInt(output));
                assert.equal(signed ? BigInt.asIntN(128, get(output)) : get(output), BigInt(Math.trunc(value)));
            }
        }
    }
    put(a, 123n); put(b, 0n);
    assert.throws(() => e.udiv(BigInt(a), BigInt(b), BigInt(output)), WebAssembly.RuntimeError);
    console.log('wasm64 wide arithmetic and conversions passed');
})().catch(error => { console.error(error); process.exitCode = 1; });

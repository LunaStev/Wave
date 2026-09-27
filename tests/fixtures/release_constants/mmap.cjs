// SPDX-License-Identifier: MPL-2.0
const fs = require('node:fs');
const assert = require('node:assert/strict');
(async () => {
  const {instance, module} = await WebAssembly.instantiate(fs.readFileSync(process.argv[2]), {});
  assert.deepEqual(WebAssembly.Module.imports(module), []);
  assert.equal(instance.exports.verify(), 0);
  assert.equal(instance.exports.verify(), 0);
})().catch(e => { console.error(e); process.exit(1); });

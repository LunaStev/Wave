// SPDX-License-Identifier: MPL-2.0
import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
const uint = n => { const a=[]; do { let b=n&127; n>>>=7; a.push(b|(n?128:0)); } while(n); return a; };
const vec = a => [...uint(a.length), ...a.flat()];
const str = s => { const b=[...Buffer.from(s)]; return [...uint(b.length),...b]; };
const section = (id,a) => [id,...uint(a.length),...a];
function moduleBytes({omit='', privateName='', wrong=false, wasi=false}={}) {
  const types = wasi ? [[0x60,0,0]] : [[0x60,2,0x7f,0x7f,1,0x7f],[0x60,1,0x7f,1,0x7f],[0x60,0,1,0x7f]];
  let exports = wasi ? [['_start',0,0],['memory',2,0]] : [['wave_add',0,0],['wave_features',0,1],['main',0,2],['memory',2,0]];
  exports=exports.filter(e=>e[0]!==omit);
  if(privateName) exports.push([privateName,0,0]);
  const bodies = wasi ? [[0,0x0b]] : [[0,0x20,0,0x20,1,0x6a,0x0b],[0,0x20,0,0x41,4,0x6c,0x0b],[0,0x41,wrong?41:42,0x0b]];
  return Buffer.from([0,97,115,109,1,0,0,0,
    ...section(1,vec(types)), ...section(3,wasi?[1,0]:[3,0,1,2]),
    ...section(5,[1,0,1]), ...section(7,vec(exports.map(([n,k,i])=>[...str(n),k,...uint(i)]))),
    ...section(10,vec(bodies.map(b=>[...uint(b.length),...b]))) ]);
}
function run(bytes,wasi=false) {
  const dir=mkdtempSync(join(tmpdir(),'wave-smoke-test-'));
  try {
    const path=join(dir,'fixture.wasm');writeFileSync(path,bytes);
    const script=fileURLToPath(new URL(wasi?'./run_wasi_smoke.mjs':'./run_wasm_smoke.mjs',import.meta.url));
    const result=spawnSync(process.execPath,['--no-warnings',script,path,dir],{encoding:'utf8',timeout:10000});
    assert.ifError(result.error);
    return result;
  } finally {rmSync(dir,{recursive:true,force:true});}
}
test('valid browser module',()=>assert.equal(run(moduleBytes()).status,0));
for (const omit of ['wave_add','wave_features','main','memory']) {
  test(`missing ${omit}`,()=>assert.notEqual(run(moduleBytes({omit})).status,0));
}
test('incorrect function result',()=>assert.notEqual(run(moduleBytes({wrong:true})).status,0));
for (const wasi of [false,true]) {
  test(`private exports rejected (wasi=${wasi})`,()=>assert.match(run(moduleBytes({wasi,privateName:'__wave_private'}),wasi).stderr,/private Wave/));
  test(`invalid module (wasi=${wasi})`,()=>assert.notEqual(run(Buffer.from('invalid'),wasi).status,0));
}
test('valid WASI command',()=>assert.equal(run(moduleBytes({wasi:true}),true).status,0));

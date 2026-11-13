import test from 'node:test';
import assert from 'node:assert/strict';
import { deflateRawSync } from 'node:zlib';
import { decodeHash, parseReport } from '../src/payload.js';
import { demo } from '../src/demo.js';
test('reads CLI JSON and core share envelope', async()=>{assert.deepEqual(parseReport(JSON.stringify(demo)),demo);const hash='#z='+deflateRawSync(JSON.stringify({...demo,gate:undefined})).toString('base64url');const loaded=await decodeHash(hash);assert.equal(loaded.diff.file.delta,313344);assert.equal(loaded.gate,null);});
test('rejects unsupported schema and inconsistent deltas',()=>{const bad=structuredClone(demo);bad.diff.schema_version=2;assert.throws(()=>parseReport(JSON.stringify(bad)));bad.diff.schema_version=1;bad.diff.file.delta=7;assert.throws(()=>parseReport(JSON.stringify(bad)));});
test('rejects invalid hashes and decompression bombs',async()=>{await assert.rejects(decodeHash('#z=!'));await assert.rejects(decodeHash('#z='+deflateRawSync(Buffer.alloc(3*1024*1024,32)).toString('base64url')),/2 MiB/);});

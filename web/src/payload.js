const MAX_BYTES = 2 * 1024 * 1024;
const object = value => value !== null && typeof value === 'object' && !Array.isArray(value);
const number = value => Number.isSafeInteger(value);
const strings = value => Array.isArray(value) && value.every(item => typeof item === 'string');
const delta = value => object(value) && typeof value.name === 'string' && number(value.base) && value.base >= 0 && number(value.head) && value.head >= 0 && number(value.delta) && value.head - value.base === value.delta;
export function validateReport(value) {
  const wrapped = object(value) && object(value.diff);
  if (wrapped && value.schema_version !== undefined && value.schema_version !== 1) throw new Error('Unsupported share schema.');
  const diff = wrapped ? value.diff : value;
  if (!object(diff) || diff.schema_version !== 1 || !['format','architecture'].every(key => typeof diff[key] === 'string') || !delta(diff.file) || !Array.isArray(diff.sections) || !diff.sections.every(delta) || !Array.isArray(diff.symbols) || !diff.symbols.every(s => delta(s) && typeof s.raw_name === 'string' && typeof s.section === 'string' && ['added','removed','modified'].includes(s.kind)) || !strings(diff.libraries_added) || !strings(diff.libraries_removed) || !strings(diff.warnings) || typeof diff.abi_complete !== 'boolean' || !Array.isArray(diff.abi) || !diff.abi.every(a => object(a) && typeof a.family === 'string' && typeof a.increased === 'boolean' && [a.base,a.head].every(v => v === null || (typeof v === 'string' && /^\d+(\.\d+)*$/.test(v))))) throw new Error('This is not a supported Vecta v1 diff report. Export with vecta diff --format json.');
  for (const key of ['omitted_symbols','omitted_sections']) if (wrapped && value[key] !== undefined && (!number(value[key]) || value[key] < 0)) throw new Error('Invalid omission counts.');
  const gate = wrapped ? value.gate : null;
  if (gate != null && (!object(gate) || typeof gate.passed !== 'boolean' || !strings(gate.violations))) throw new Error('Invalid gate result.');
  return { diff, gate: gate ?? null, omitted_symbols: wrapped ? value.omitted_symbols ?? 0 : 0, omitted_sections: wrapped ? value.omitted_sections ?? 0 : 0 };
}
export function parseReport(text) {
  if (new TextEncoder().encode(text).length > MAX_BYTES) throw new Error('Report exceeds the 2 MiB viewer limit.');
  return validateReport(JSON.parse(text));
}
export async function decodeHash(hash) {
  if (!hash.startsWith('#z=')) throw new Error('Expected a Vecta URL with a #z= fragment.');
  const encoded = hash.slice(3);
  if (!encoded || encoded.length > 8000 || !/^[A-Za-z0-9_-]+$/.test(encoded) || encoded.length % 4 === 1) throw new Error('The share URL is malformed or too large.');
  if (typeof DecompressionStream === 'undefined') throw new Error('This browser cannot decode shared reports. Open a JSON report instead.');
  const bytes = Uint8Array.from(atob(encoded.replace(/-/g,'+').replace(/_/g,'/')), c => c.charCodeAt(0));
  let stream;
  try { stream = new DecompressionStream('deflate-raw'); } catch { throw new Error('Raw DEFLATE is unavailable in this browser. Open a JSON report instead.'); }
  const reader = new Blob([bytes]).stream().pipeThrough(stream).getReader();
  const chunks = []; let size = 0;
  try {
    while (true) { const {done,value} = await reader.read(); if (done) break; size += value.length; if (size > MAX_BYTES) { await reader.cancel(); throw new Error('Decompressed report exceeds the 2 MiB viewer limit.'); } chunks.push(value); }
  } finally { reader.releaseLock(); }
  const result = new Uint8Array(size); let offset = 0;
  for (const chunk of chunks) { result.set(chunk,offset); offset += chunk.length; }
  return parseReport(new TextDecoder('utf-8',{fatal:true}).decode(result));
}

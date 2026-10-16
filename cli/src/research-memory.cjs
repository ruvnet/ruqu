'use strict';
// Local measured evidence and advisory retrieval. No simulator or provider is invoked.
const fs = require('node:fs/promises');
const crypto = require('node:crypto');
const SCHEMA = 'ruqu-research-memory/v1';
const MAX_RECORDS = 10000, MAX_BYTES = 16 * 1024 * 1024, DIM = 8;
const hash = value => crypto.createHash('sha256').update(value).digest('hex');
function canonical(value) {
  if (value === null || typeof value === 'boolean' || typeof value === 'string') return JSON.stringify(value);
  if (typeof value === 'number' && Number.isFinite(value)) return JSON.stringify(value);
  if (Array.isArray(value)) return '[' + value.map(canonical).join(',') + ']';
  if (value && Object.getPrototypeOf(value) === Object.prototype) {
    const keys = Object.keys(value).sort();
    if (keys.some(k => ['__proto__', 'constructor', 'prototype'].includes(k))) throw Error('Unsafe object key');
    return '{' + keys.map(k => JSON.stringify(k) + ':' + canonical(value[k])).join(',') + '}';
  }
  throw Error('Only finite JSON data is supported');
}
function finite(n, name, min = 0, max = Infinity) {
  if (!Number.isFinite(n) || n < min || n > max) throw Error('Invalid ' + name);
}
function validSeed(seed) {
  if (typeof seed === 'number') return Number.isSafeInteger(seed) && seed >= 0;
  return typeof seed === 'string' && /^(0|[1-9][0-9]{0,19})$/.test(seed) && BigInt(seed) <= 18446744073709551615n;
}
function targetCheck(t) {
  canonical(t);
  if (!t || !/^[a-f0-9]{64}$/.test(t.circuitSha256) || !t.noise || !t.budget ||
      typeof t.backend !== 'string' || !t.backend || t.backend.length > 100 ||
      !validSeed(t.seed) || !t.environment || typeof t.environment !== 'object') throw Error('Invalid target identity');
  for (const field of ['physicalErrorRate', 'p']) if (t.noise[field] !== undefined) finite(t.noise[field], 'physical error probability', 0, 1);
  finite(t.budget.maxRuntimeMs, 'runtime budget', 0, 86400000);
  finite(t.budget.maxMemoryBytes, 'memory budget', 0, 2 ** 53 - 1);
  finite(t.budget.maxOutputError, 'error budget', 0, 1);
  finite(t.budget.minAcceptance, 'acceptance budget', 0, 1);
}
function exactKey(target) { targetCheck(target); return hash(canonical(target)); }
function createEvidence({ target, metrics, provenance }) {
  const record = { schema: SCHEMA, target, metrics, provenance, key: exactKey(target) };
  record.digest = hash(canonical(record));
  validateEvidence(record, { allowStale: true });
  return JSON.parse(canonical(record));
}
function validateEvidence(r, { now = Date.now(), maxAgeMs = 30 * 86400000, allowStale = false } = {}) {
  if (!r || r.schema !== SCHEMA || r.key !== exactKey(r.target)) throw Error('Evidence identity mismatch');
  const { digest, ...body } = r;
  if (digest !== hash(canonical(body))) throw Error('Evidence digest mismatch');
  const m = r.metrics, p = r.provenance;
  if (!m || !p || !['native-measured','reference-exact','synthetic-test'].includes(p.kind) ||
      typeof p.runId !== 'string' || !p.runId || p.runId.length > 200 ||
      typeof p.toolVersion !== 'string' || !p.toolVersion || !/^[a-f0-9]{64}$/.test(p.sourceSha256)) throw Error('Missing evidence provenance');
  const at = Date.parse(p.measuredAt);
  if (!Number.isFinite(at) || at > now + 60000) throw Error('Invalid measurement time');
  if (!allowStale && now - at > maxAgeMs) throw Error('Stale evidence');
  finite(m.runtimeMs, 'runtime'); finite(m.peakMemoryBytes, 'memory');
  finite(m.outputError, 'output error', 0, 1);
  for (const k of ['attempts','accepted','rejected']) {
    if (!Number.isSafeInteger(m[k]) || m[k] < 0) throw Error('Invalid attempt count');
  }
  if (!m.attempts || m.accepted + m.rejected !== m.attempts) throw Error('Unaccounted attempts');
  return r;
}
function withinBudget(r) {
  const b = r.target.budget, m = r.metrics;
  return m.runtimeMs <= b.maxRuntimeMs && m.peakMemoryBytes <= b.maxMemoryBytes &&
    m.outputError <= b.maxOutputError && m.accepted / m.attempts >= b.minAcceptance;
}
function embedding(t) {
  // Structural retrieval features, not a learned embedding or accuracy prediction.
  const f = t.features || {}, n = t.noise || {};
  return Float32Array.from([f.qubits || 0, f.gates || 0, f.tGates || 0, f.measurements || 0,
    n.physicalErrorRate || n.p || 0, t.budget.maxOutputError, t.budget.minAcceptance,
    Math.log1p(t.budget.maxRuntimeMs)], (x, i) => {
      finite(x, 'retrieval feature'); return i < 4 ? Math.log1p(x) : x;
    });
}
function validRecords(records, opts) {
  if (!Array.isArray(records) || records.length > MAX_RECORDS) throw Error('Evidence count exceeds limit');
  const seen = new Set();
  return records.map(r => {
    validateEvidence(r, opts);
    if (seen.has(r.provenance.runId)) throw Error('Duplicate/replayed run ID');
    seen.add(r.provenance.runId); return r;
  });
}
function lookupExact(records, target, opts = {}) {
  const key = exactKey(target);
  return validRecords(records, opts).filter(r => r.key === key && r.provenance.kind === 'native-measured' && withinBudget(r))
    .sort((a,b) => Date.parse(b.provenance.measuredAt)-Date.parse(a.provenance.measuredAt))[0] || null;
}
function recommend(records, target, opts = {}) {
  targetCheck(target);
  const candidates = [], skipped = [];
  for (const r of validRecords(records, { ...opts, allowStale: true })) {
    try { validateEvidence(r, opts); } catch (e) { skipped.push({ key: r.key, reason: e.message }); continue; }
    if (r.provenance.kind !== 'native-measured' || !withinBudget({ ...r, target: { ...r.target, budget: target.budget } })) continue;
    const a = embedding(target), b = embedding(r.target);
    const distance = a.reduce((s, x, i) => s + (x-b[i])**2, 0);
    candidates.push({ key: r.key, backend: r.target.backend, exact: r.key === exactKey(target), distance,
      runtimeMs: r.metrics.runtimeMs, acceptance: r.metrics.accepted/r.metrics.attempts });
  }
  candidates.sort((a,b) => Number(b.exact)-Number(a.exact) || a.distance-b.distance || a.runtimeMs-b.runtimeMs);
  return { advisoryOnly: true, selectedBackend: target.backend, candidates: candidates.slice(0, 10), skipped };
}
function checkPromotion(pairs, { minPairs = 2, minImprovement = 0.1, maxErrorDelta = 0, locallyValidatedEvidence = [], locallyValidatedRunIds, ...opts } = {}) {
  if (!Number.isSafeInteger(minPairs) || minPairs < 2 || minPairs > MAX_RECORDS) return { eligible: false, reason: 'Invalid holdout pair bound' };
  if (!Array.isArray(pairs) || pairs.length < minPairs || pairs.length > MAX_RECORDS) return { eligible: false, reason: 'At least two paired holdout seeds required' };
  finite(minImprovement, 'minimum improvement', 0, 1); finite(maxErrorDelta, 'error tolerance', 0, 1);
  if (locallyValidatedRunIds !== undefined) return { eligible: false, reason: 'Run ID-only approval is unsupported; approve exact evidence digests' };
  if (!Array.isArray(locallyValidatedEvidence) || locallyValidatedEvidence.length > MAX_RECORDS) return { eligible: false, reason: 'Invalid local evidence ledger' };
  const allowed = new Map(), ids = new Set(), seeds = new Set();
  for (const entry of locallyValidatedEvidence) {
    if (!entry || typeof entry.runId !== 'string' || !entry.runId || !/^[a-f0-9]{64}$/.test(entry.digest) || allowed.has(entry.runId))
      return { eligible: false, reason: 'Local ledger requires unique run IDs and SHA256 evidence digests' };
    allowed.set(entry.runId, entry.digest);
  }
  let baseline = 0, candidate = 0, backends;
  try {
    for (const pair of pairs) {
      const a = validateEvidence(pair.baseline, opts), b = validateEvidence(pair.candidate, opts);
      const identity = r => { const { backend, ...t } = r.target; return canonical(t); };
      if (identity(a) !== identity(b) || a.target.backend === b.target.backend) throw Error('Paired targets must match except backend');
      if (backends && backends !== canonical([a.target.backend,b.target.backend])) throw Error('Inconsistent backend pair');
      backends = canonical([a.target.backend,b.target.backend]);
      if (seeds.has(String(a.target.seed))) throw Error('Holdout seeds must be distinct');
      seeds.add(String(a.target.seed));
      for (const r of [a,b]) {
        if (r.provenance.kind !== 'native-measured' || r.provenance.partition !== 'holdout' || allowed.get(r.provenance.runId) !== r.digest) throw Error('Locally validated native holdout evidence with exact approved digest required');
        if (ids.has(r.provenance.runId)) throw Error('Replayed run ID'); ids.add(r.provenance.runId);
      }
      if (!withinBudget(b) || b.metrics.outputError > a.metrics.outputError + maxErrorDelta ||
          b.metrics.accepted/b.metrics.attempts < a.metrics.accepted/a.metrics.attempts) throw Error('Accuracy, yield or budget regression');
      baseline += a.metrics.runtimeMs; candidate += b.metrics.runtimeMs;
    }
    if (baseline <= 0 || candidate > baseline * (1-minImprovement)) throw Error('Insufficient measured runtime improvement');
    return { eligible: true, advisoryOnly: true, improvement: 1-candidate/baseline, pairs: pairs.length };
  } catch (e) { return { eligible: false, reason: e.message }; }
}
async function wasm() {
  const { default: init } = await import('@ruvector/rvf-wasm');
  return init();
}
function allocated(w, bytes, fn) {
  const ptr = w.rvf_alloc(bytes.length);
  if (!ptr) throw Error('RVF allocation failed');
  try { new Uint8Array(w.memory.buffer, ptr, bytes.length).set(bytes); return fn(ptr); }
  finally { w.rvf_free(ptr, bytes.length); }
}
async function exportMemory(records, output, opts = {}) {
  validRecords(records, opts);
  if (!records.length || records.some(r => r.provenance.kind !== 'native-measured')) throw Error('Export requires native measured evidence');
  const w = await wasm(), h = w.rvf_store_create(DIM, 0);
  if (h <= 0) throw Error('RVF create failed');
  let bytes;
  try {
    const vectors = new Float32Array(records.length*DIM), ids = new BigUint64Array(records.length);
    records.forEach((r,i) => { vectors.set(embedding(r.target), i*DIM); ids[i] = BigInt(i+1); });
    const accepted = allocated(w, new Uint8Array(vectors.buffer), vp => allocated(w, new Uint8Array(ids.buffer), ip => w.rvf_store_ingest(h,vp,ip,records.length)));
    if (accepted !== records.length) throw Error('RVF ingest count mismatch');
    const probe = w.rvf_store_export(h,0,0), size = Math.abs(probe);
    if (!size || size > MAX_BYTES) throw Error('RVF export size exceeds limit');
    const ptr = w.rvf_alloc(size);
    try { const count = w.rvf_store_export(h,ptr,size); if (count !== size) throw Error('RVF export size mismatch'); bytes = Buffer.from(new Uint8Array(w.memory.buffer,ptr,size)); }
    finally { w.rvf_free(ptr,size); }
  } finally { w.rvf_store_close(h); }
  const body = { schema: SCHEMA, rvfSha256: hash(bytes), dimensions: DIM, records };
  const manifest = { ...body, digest: hash(canonical(body)) };
  if (Buffer.byteLength(canonical(manifest)) > MAX_BYTES) throw Error('Manifest exceeds limit');
  await fs.writeFile(output,bytes,{flag:'wx'});
  try { await fs.writeFile(output+'.json',canonical(manifest)+'\n',{flag:'wx'}); }
  catch(e) { await fs.unlink(output); throw e; }
  return { output, manifest: output+'.json', records: records.length, rvfSha256: body.rvfSha256 };
}
async function boundedRead(file) {
  const handle = await fs.open(file,'r');
  try {
    if ((await handle.stat()).size > MAX_BYTES) throw Error('File exceeds size limit');
    const bytes = Buffer.alloc(MAX_BYTES + 1); let offset = 0;
    while (offset < bytes.length) { const { bytesRead } = await handle.read(bytes, offset, bytes.length-offset, null); if (!bytesRead) break; offset += bytesRead; }
    if (offset > MAX_BYTES) throw Error('File exceeds size limit');
    return bytes.subarray(0,offset);
  }
  finally { await handle.close(); }
}
async function importMemory(input, opts = {}) {
  const bytes = await boundedRead(input), manifest = JSON.parse(await boundedRead(input+'.json'));
  const { digest, ...body } = manifest;
  if (manifest.schema !== SCHEMA || manifest.dimensions !== DIM || digest !== hash(canonical(body)) || manifest.rvfSha256 !== hash(bytes)) throw Error('Memory integrity mismatch');
  validRecords(manifest.records, opts);
  if (manifest.records.some(r => r.provenance.kind !== 'native-measured')) throw Error('Imported evidence is not native measured');
  const w = await wasm();
  const h = allocated(w,bytes,p => w.rvf_store_open(p,bytes.length));
  if (h <= 0) throw Error('Invalid RVF store');
  try { if (w.rvf_store_count(h) !== manifest.records.length || w.rvf_store_dimension(h) !== DIM) throw Error('RVF shape mismatch'); }
  finally { w.rvf_store_close(h); }
  return { records: manifest.records, trustedForPromotion: false, rvfSha256: manifest.rvfSha256 };
}
async function queryMemory(input, target, { k = 5, ...opts } = {}) {
  targetCheck(target); if (!Number.isInteger(k) || k < 1 || k > 100) throw Error('Invalid retrieval bound');
  const memory = await importMemory(input,opts), bytes = await boundedRead(input), w = await wasm();
  // Recheck bytes after reopening to avoid querying a file replaced after validation.
  if (hash(bytes) !== memory.rvfSha256) throw Error('Memory changed during query');
  const h = allocated(w,bytes,p => w.rvf_store_open(p,bytes.length));
  if (h <= 0) throw Error('RVF open failed');
  try {
    const vector = embedding(target), outSize = k*12, ptr = w.rvf_alloc(outSize);
    try {
      const count = allocated(w,new Uint8Array(vector.buffer),p => w.rvf_store_query(h,p,k,0,ptr));
      if (count < 0 || count > k) throw Error('RVF query failed');
      const view = new DataView(w.memory.buffer), results = [];
      for (let i=0;i<count;i++) {
        const id = Number(view.getBigUint64(ptr+i*12,true)), record = memory.records[id-1];
        if (!record) throw Error('RVF vector identity mismatch');
        const distance = view.getFloat32(ptr+i*12+8,true);
        finite(distance, 'RVF query distance');
        results.push({ record, distance });
      }
      return { advisoryOnly: true, selectedBackend: target.backend, results };
    } finally { w.rvf_free(ptr,outSize); }
  } finally { w.rvf_store_close(h); }
}
async function run(args) {
  const command = args[0], option = name => { const i=args.indexOf('--'+name); return i<0 ? undefined : args[i+1]; };
  const input = option('input');
  if (!input) throw Error('research-memory requires --input');
  if (command === 'export') {
    if (!option('output')) throw Error('export requires --output');
    return exportMemory(JSON.parse(await boundedRead(input)),option('output'));
  }
  if (command === 'import') return importMemory(input);
  if (command === 'query' || command === 'recommend') {
    if (!option('target')) throw Error('query/recommend requires --target');
    const target = JSON.parse(await boundedRead(option('target')));
    return command === 'query' ? queryMemory(input,target) : recommend(JSON.parse(await boundedRead(input)),target);
  }
  throw Error('Expected export, import, query or recommend');
}
module.exports = { SCHEMA, canonical, exactKey, createEvidence, validateEvidence, withinBudget, lookupExact, recommend, checkPromotion, exportMemory, importMemory, queryMemory, run, runCLI: run };
if (require.main === module) run(process.argv.slice(2)).then(x=>console.log(JSON.stringify(x,null,2))).catch(e=>{console.error(e.message);process.exitCode=1;});

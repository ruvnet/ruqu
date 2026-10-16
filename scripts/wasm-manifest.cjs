'use strict';
const fs=require('node:fs'), path=require('node:path'), crypto=require('node:crypto'), assert=require('node:assert/strict');
const root=path.resolve(__dirname,'..');
const paths=['Cargo.lock','crates/ruqu-core/Cargo.toml','crates/ruqu-wasm/Cargo.toml'];
function walk(dir){for(const e of fs.readdirSync(path.join(root,dir),{withFileTypes:true})){const p=dir+'/'+e.name;if(e.isDirectory())walk(p);else if(p.endsWith('.rs'))paths.push(p);}}
for(const p of ['crates/ruqu-core/src','crates/ruqu-algorithms/src','crates/ruqu-wasm/src'])walk(p);
for(const dir of ['cli/wasm','explorer/wasm','npm/packages/ruqu-wasm'])for(const f of ['ruqu_wasm.js','ruqu_wasm.d.ts','ruqu_wasm_bg.wasm','ruqu_wasm_bg.wasm.d.ts'])paths.push(dir+'/'+f);
const hashes=Object.fromEntries(paths.sort().map(p=>[p,crypto.createHash('sha256').update(fs.readFileSync(path.join(root,p))).digest('hex')]));
assert.equal(hashes['cli/wasm/ruqu_wasm_bg.wasm'],hashes['explorer/wasm/ruqu_wasm_bg.wasm']);
assert.equal(hashes['explorer/wasm/ruqu_wasm_bg.wasm'],hashes['npm/packages/ruqu-wasm/ruqu_wasm_bg.wasm']);
const file=path.join(root,'docs/research/wasm-manifest.json');
if(process.argv.includes('--write'))fs.writeFileSync(file,JSON.stringify({schema_version:1,rust:'1.90.0',wasm_bindgen:'0.2.125',sha256:hashes},null,2)+'\n');
else assert.deepEqual(JSON.parse(fs.readFileSync(file)).sha256,hashes,'WASM source or artifacts changed: rebuild with scripts/build-wasm.sh');
console.log('WASM source and three distribution hashes verified');

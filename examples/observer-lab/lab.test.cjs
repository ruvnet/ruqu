const test=require('node:test');
const assert=require('node:assert/strict');
const {bell,chsh,localBound,observer,makeLedger,verify,run,rng}=require('./lab.cjs');
const near=(a,b)=>assert.ok(Math.abs(a-b)<1e-10, `${a} != ${b}`);
test('Bell Z outcomes agree with analytic distribution',()=>{
  bell(0,0).forEach((v,i)=>near(v,[0.5,0,0,0.5][i]));
});
test('CHSH matches analytic quantum prediction',()=>near(chsh().S,2*Math.sqrt(2)));
test('all 16 local deterministic strategies obey bound',()=>assert.equal(localBound(),2));
test('dephased control loses this CHSH violation',()=>near(chsh(true).S,Math.sqrt(2)));
test('coherent observer record removes local interference',()=>near(observer(false).systemZero,0.5));
test('uncomputing observer restores interference',()=>near(observer(true).systemZero,1));
test('local marginals independent of remote setting',()=>{
  for(const a of [0,0.3,Math.PI/2]) for(const b of [0,0.7,Math.PI/4]) {
    const p=bell(a,b); near(p[0]+p[2],0.5); near(p[0]+p[1],0.5);
  }
});
test('seeded records replay exactly',()=>assert.deepEqual(makeLedger(),makeLedger()));
test('valid ledger verifies',()=>{const l=makeLedger(); assert.ok(verify(l.rows,l.root));});
test('altered evidence rejected',()=>{
  const l=makeLedger(); l.rows[0].record.alice^=1; assert.equal(verify(l.rows,l.root),false);
});
test('truncated evidence rejected against trusted root',()=>{
  const l=makeLedger(); assert.equal(verify(l.rows.slice(0,-1),l.root),false);
});
test('memory intervention leaves evidence unchanged',()=>{
  const r=run(); near(r.disagreementFraction,0.25); assert.ok(verify(r.ledger.rows,r.ledger.root));
  assert.ok(r.ledger.rows.every(x=>x.record.alice===x.record.bob));
});
test('invalid resource limits and seeds rejected',()=>{
  for(const n of [-1,0,100001,1.5]) assert.throws(()=>makeLedger(n));
  for(const seed of [0,-1,NaN,Infinity]) assert.throws(()=>rng(seed));
});

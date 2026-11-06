const assert=require('node:assert/strict');
const {test}=require('node:test');
const {resolve}=require('node:path');
const {probe}=require('../scripts/parser-boundary-probe.cjs');
test('parser errors preserve HTTP status and response envelope',async()=>{
 const result=await probe(resolve(__dirname,'..'));
 for(const row of [...result.holdout,...result.anchors])assert.equal(row.passed,true,JSON.stringify(row));
});

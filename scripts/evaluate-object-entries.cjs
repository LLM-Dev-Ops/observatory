const fs=require('node:fs');
const {spawnSync,execFileSync}=require('node:child_process');
const {resolve,join}=require('node:path');
const {createHash}=require('node:crypto');
const [base,head,tools,out]=process.argv.slice(2).map(resolvePath=>resolve(resolvePath));
const hash=x=>createHash('sha256').update(x).digest('hex');
const frozen=['scripts/object-entry-probe.cjs','functions/object-entry.test.js','docs/metaharness/2026-11-01/contract.json'];
for(const path of frozen) if(!fs.readFileSync(join(base,path)).equals(fs.readFileSync(join(head,path)))) throw Error('Frozen bytes changed: '+path);
fs.mkdirSync(out+'.logs',{recursive:true});
function run(root,label,key,command,args,seconds=45){
  const start=Date.now();
  const r=spawnSync('timeout',['--kill-after=5',String(seconds),command,...args],{cwd:root,encoding:'utf8',maxBuffer:32*1024*1024,timeout:(seconds+10)*1000,env:{...process.env,CARGO_TARGET_DIR:'/tmp/observatory-nov01a-target',CARGO_PROFILE_DEV_DEBUG:'0',CARGO_PROFILE_TEST_DEBUG:'0'}});
  const stdout=r.stdout||'',stderr=r.stderr||'';
  fs.writeFileSync(join(out+'.logs',label+'-'+key+'.log'),stdout+stderr);
  return {command:['timeout','--kill-after=5',String(seconds),command,...args],exit:r.status,signal:r.signal,error:r.error?.message,ms:Date.now()-start,sha256:hash(stdout+stderr),pass:r.status===0&&!r.error&&!r.signal,stdout};
}
function measure(root,label){
  const checks={syntax:run(root,label,'syntax',process.execPath,['--check','functions/index.js'],10),probe:run(root,label,'probe',process.execPath,['scripts/object-entry-probe.cjs','.'],15),node:run(root,label,'node','npm',['test'],30)};
  const probe=JSON.parse(checks.probe.stdout);
  for(const op of ['test','build','clippy']) checks[op]=run(root,label,op,'cargo',['+1.88.0',op,'--workspace','--all-features','--locked','--offline','-j2',...(op==='clippy'?['--all-targets','--','-D','warnings']:[])]);
  checks.fmt=run(root,label,'fmt','cargo',['+1.88.0','fmt','--all','--','--check'],30);
  checks.audit=run(root,label,'audit','cargo',['audit','--db','/home/ruv/.cargo/advisory-db','--file','Cargo.lock','--no-fetch','--json'],30);
  checks.npmAudit=run(root,label,'npm-audit','npm',['audit','--omit=dev','--json'],30);
  return {probe,checks};
}
(async()=>{
  const baseline=measure(base,'baseline'),candidate=measure(head,'candidate');
  const fw=await import(join(tools,'node_modules/@metaharness/flywheel/dist/index.js'));
  const rev=root=>execFileSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).trim();
  const b=rev(base),h=rev(head),parentBytes=fs.readFileSync(join(base,'functions/index.js'),'utf8');
  const score=(m,s)=>{const items=m.probe[s==='holdout'?'holdout':'anchors'];const primary=items.filter(x=>x.passed).length/items.length;return {primary,noopRate:1-primary,costPerWin:0,regressed:Object.values(m.checks).some(x=>!x.pass)};};
  const result=await fw.runFlywheelGenerations({rootPolicy:{revision:b},proposer:async()=>({value:h,summary:'Validate object entries before health/SLO property access',inverse:{path:'functions/index.js',parentBytes,sha256:hash(parentBytes)}}),evaluator:async(p,s)=>score(p.revision===b?baseline:candidate,s.id),holdout:{id:'holdout',items:candidate.probe.holdout.map(x=>x.name)},anchor:{id:'anchor',items:candidate.probe.anchors.map(x=>x.name)},mutationTargets:['revision'],maxGenerations:1,signer:fw.makeSigner(),promotionRule:fw.meetsPromotionRule,dataSource:'LIVE'});
  const fingerprint=fw.gateFingerprint(fw.meetsPromotionRule),replay=fw.verifyReplayBundle(result.replayBundle,{pinnedGateFingerprint:fingerprint});
  for(const m of [baseline,candidate]) for(const c of Object.values(m.checks)) delete c.stdout;
  fs.writeFileSync(out,JSON.stringify({baselineRevision:b,candidateRevision:h,frozen: frozen.map(path=>({path,sha256:hash(fs.readFileSync(join(head,path)))})),baseline,candidate,result,fingerprint,replay,rollback:{path:'functions/index.js',parentBytes,sha256:hash(parentBytes)},limitations:'Real loopback HTTP, not deployed IAM/integration. No cost or latency claim; costPerWin=0 is neutral unmeasured and noopRate is failed-case fraction. Broad failures are regressions for promotion even when pre-existing.'},null,2));
  console.log(JSON.stringify({promotions:result.promotions.length,replay,baseline:score(baseline,'holdout'),candidate:score(candidate,'holdout')}));
})().catch(error=>{console.error(error);process.exitCode=1;});

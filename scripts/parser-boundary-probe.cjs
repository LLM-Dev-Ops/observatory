const {createServer}=require('node:http');
const {resolve}=require('node:path');
async function probe(root){
 const {api}=require(resolve(root,'functions/index.js'));
 const server=createServer(api);
 await new Promise(r=>server.listen(0,'127.0.0.1',r));
 const origin='http://127.0.0.1:'+server.address().port;
 async function check(name,path,raw,status,verify,method='POST'){
  const response=await fetch(origin+path,{method,headers:{'content-type':'application/json'},...(raw===undefined?{}:{body:raw})});
  const body=await response.json();
  return {name,status:response.status,passed:response.status===status&&verify(body,response),body};
 }
 try{
  const holdout=[];
  for(const [name,raw] of [['malformed','{"secret":"PRIVATE_MARKER",}'],['truncated','{"payload":'],['null','null'],['number','7'],['string','"PRIVATE_MARKER"']]){
   holdout.push(await check(name,'/v1/observatory/telemetry',raw,400,(b,r)=>b.error?.code==='INVALID_JSON'&&b.success===false&&b.layers_executed?.join()==='input_validation'&&!!b.execution_metadata?.trace_id&&r.headers.get('access-control-allow-origin')==='*'&&!JSON.stringify(b).includes('PRIVATE_MARKER')));
  }
  holdout.push(await check('oversized','/v1/observatory/telemetry',JSON.stringify({payload:{events:['x'.repeat(10*1024*1024)]}}),413,b=>b.error?.code==='PAYLOAD_TOO_LARGE'&&b.success===false&&b.layers_executed?.join()==='input_validation'&&!!b.execution_metadata?.trace_id));
  const anchors=[
   await check('health','/health',undefined,200,b=>b.healthy===true&&!!b.execution_metadata?.trace_id,'GET'),
   await check('telemetry','/v1/observatory/telemetry',JSON.stringify({payload:{events:[{id:'ok'}]}}),200,b=>b.processed===1&&b.decision_event.outputs.processed===1),
   await check('shape validation','/v1/observatory/telemetry','{}',400,b=>b.error?.code==='VALIDATION_FAILED'),
   await check('missing route','/absent','{}',404,b=>b.error?.code==='NOT_FOUND'),
   await check('health targets','/v1/observatory/health-check',JSON.stringify({payload:{targets:[{id:'a'}]}}),200,b=>b.evaluations[0].target_id==='a'),
   await check('SLO fields','/v1/observatory/slo',JSON.stringify({payload:{slo_definitions:[{id:'a',threshold:99}],metrics:[]}}),200,b=>b.slo_statuses[0].threshold===99)
  ];
  return {holdout,anchors};
 }finally{server.closeAllConnections();await new Promise(r=>server.close(r));}
}
module.exports={probe};
if(require.main===module)probe(process.argv[2]||'.').then(x=>console.log(JSON.stringify(x))).catch(e=>{console.error(e);process.exitCode=1;});

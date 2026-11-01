const {createServer} = require('node:http');
const {resolve} = require('node:path');

async function probe(root) {
  const {api} = require(resolve(root, 'functions/index.js'));
  const server = createServer(api);
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const origin = `http://127.0.0.1:${server.address().port}`;
  async function check(name, route, payload, expected, verify = () => true) {
    const response = await fetch(origin + '/v1/observatory/' + route, {
      method: 'POST', headers: {'content-type': 'application/json'}, body: JSON.stringify({payload}),
    });
    const body = await response.json();
    return {name, status: response.status, passed: response.status === expected && verify(body)};
  }
  try {
    const holdout = [];
    for (const [route, field] of [['health-check', 'targets'], ['slo', 'slo_definitions']]) {
      for (const value of [null, 7, []]) {
        holdout.push(await check(`${route}:${JSON.stringify(value)}`, route, {[field]: [{id: 'ok'}, value]}, 400,
          body => body.error?.code === 'VALIDATION_FAILED' && body.layers_executed?.join() === 'input_validation'));
      }
    }
    const anchors = [
      await check('health aliases', 'health-check', {targets: [{target_id:'a', target_type:'service'}]}, 200,
        body => body.evaluations?.[0]?.target_id === 'a' && body.evaluations[0].target_type === 'service'),
      await check('SLO threshold', 'slo', {slo_definitions:[{id:'a', threshold:99}], metrics:[1]}, 200,
        body => body.slos_evaluated === 1 && body.slo_statuses[0].threshold === 99),
      await check('empty health targets', 'health-check', {targets:[]}, 200, body => body.evaluations.length === 0),
    ];
    return {holdout, anchors};
  } finally { server.closeAllConnections(); await new Promise(resolve => server.close(resolve)); }
}
module.exports = {probe};
if (require.main === module) probe(process.argv[2] || '.').then(result => console.log(JSON.stringify(result))).catch(error => {console.error(error); process.exitCode=1;});

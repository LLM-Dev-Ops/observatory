const assert = require('node:assert/strict');
const { after, before, test } = require('node:test');
const { createServer } = require('node:http');
const { api } = require('./index');

let server;
let baseUrl;

before(async () => {
  server = createServer(api);
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  baseUrl = `http://127.0.0.1:${server.address().port}`;
});

after(async () => {
  await new Promise((resolve, reject) => {
    server.close((error) => (error ? reject(error) : resolve()));
  });
});

async function request(path, body) {
  const response = await fetch(`${baseUrl}${path}`, {
    method: body === undefined ? 'GET' : 'POST',
    headers: { 'content-type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  return { status: response.status, body: await response.json() };
}

test('preserves health and valid telemetry behavior', async () => {
  const health = await request('/health');
  assert.equal(health.status, 200);
  assert.equal(health.body.healthy, true);
  assert.ok(health.body.execution_metadata.trace_id);

  const telemetry = await request('/v1/observatory/telemetry', {
    payload: { events: [{ id: 'event-1' }] },
  });
  assert.equal(telemetry.status, 200);
  assert.equal(telemetry.body.processed, 1);
});

test('rejects missing telemetry events', async () => {
  const result = await request('/v1/observatory/telemetry', { payload: {} });
  assert.equal(result.status, 400);
  assert.equal(result.body.error.code, 'VALIDATION_FAILED');
});

for (const [name, path, body] of [
  ['telemetry events', '/v1/observatory/telemetry', { payload: { events: {} } }],
  ['failure events', '/v1/observatory/failures', { payload: { error_events: {} } }],
  ['health targets', '/v1/observatory/health-check', { payload: { targets: {} } }],
  ['SLO definitions', '/v1/observatory/slo', { payload: { slo_definitions: {} } }],
  ['SLO metrics', '/v1/observatory/slo', { payload: { metrics: {} } }],
]) {
  test(`rejects non-array ${name}`, async () => {
    const result = await request(path, body);
    assert.equal(result.status, 400);
    assert.equal(result.body.error.code, 'VALIDATION_FAILED');
  });
}

test('rejects oversized telemetry batches before processing', async () => {
  const result = await request('/v1/observatory/telemetry', {
    payload: { events: Array.from({ length: 1001 }, (_, id) => ({ id })) },
  });
  assert.equal(result.status, 413);
  assert.equal(result.body.error.code, 'PAYLOAD_TOO_LARGE');
});

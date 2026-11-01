const assert = require('node:assert/strict');
const {test} = require('node:test');
const {probe} = require('../scripts/object-entry-probe.cjs');
const {resolve} = require('node:path');

test('rejects invalid object entries and preserves valid batch behavior', async () => {
  const results = await probe(resolve(__dirname, '..'));
  for (const result of [...results.holdout, ...results.anchors]) assert.equal(result.passed, true, JSON.stringify(result));
});

import assert from 'node:assert/strict';
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { performance } from 'node:perf_hooks';
import { probeMigrations } from './probe-migrations.mjs';

// Five sequential fresh-database trials per revision; same immutable fixture.
// Baseline stops at 005, candidate executes 001-008: timings are NOT speedup data.
const [baseline, candidate, output] = process.argv.slice(2).map(x => resolve(x));
const contract = JSON.parse(readFileSync(new URL('../docs/metaharness/2026-10-18/contract.json', import.meta.url)));
const results = [];
for (const root of [baseline, candidate]) {
  const trials = [];
  for (let trial = 0; trial < 5; trial++) {
    const start = performance.now();
    const report = probeMigrations(root, contract);
    const elapsedMs = performance.now() - start;
    assert(report.anchor.every(Boolean));
    assert.equal(report.cleanup.exit, 0);
    trials.push({ trial: trial + 1, elapsedMs, fullChainPassed: report.holdout.every(Boolean), applied: report.migrations.filter(x => x.exit === 0).length, holdout: report.holdout, anchor: report.anchor, versions: report.versions, cleanup: report.cleanup });
  }
  const sorted = trials.map(x => x.elapsedMs).sort((a,b) => a-b);
  results.push({ root, completeChains: trials.filter(x => x.fullChainPassed).length, medianMs: sorted[2], minimumMs: sorted[0], maximumMs: sorted[4], trials });
}
assert.equal(results[0].completeChains, 0); assert.equal(results[1].completeChains, 5);
const report = { protocol: 'Five sequential fresh ephemeral databases per revision; actual full SQL execution and catalog/data assertions, same fixture and server; no retries or discarded trials.', results, interpretation: 'Functional reliability benchmark: baseline 0/5 complete chains versus candidate 5/5. Baseline stops on migration 005 and does less work, so elapsed times are incomparable and no performance improvement is claimed.' };
writeFileSync(output, JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify({ results: results.map(({ trials, ...summary }) => summary), interpretation: report.interpretation }));

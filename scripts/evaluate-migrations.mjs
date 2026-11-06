import assert from 'node:assert/strict';
import { readFileSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { resolve, join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { probeMigrations, hash } from './probe-migrations.mjs';

// node scripts/evaluate-migrations.mjs BASELINE CANDIDATE PINNED_TOOLS OUTPUT
const [baseline, candidate, tools, output] = process.argv.slice(2).map(x => resolve(x));
const fw = await import(pathToFileURL(join(tools, 'node_modules/@metaharness/flywheel/dist/index.js')));
assert.equal(JSON.parse(readFileSync(join(tools, 'node_modules/@metaharness/flywheel/package.json'))).version, '0.1.12');
const source = 'crates/storage/migrations/005_retention_policies.sql';
const sourceHash = root => hash(readFileSync(join(root, source)));
const parentHash = sourceHash(baseline), candidateHash = sourceHash(candidate);
assert.notEqual(parentHash, candidateHash);
const contractBytes = readFileSync(new URL('../docs/metaharness/2026-10-18/contract.json', import.meta.url));
const contract = JSON.parse(contractBytes);
const measurements = [];
let retainedNames;
async function evaluator(policy, suite) {
  const root = policy.retention === parentHash ? baseline : candidate;
  assert.equal(sourceHash(root), policy.retention);
  const report = probeMigrations(root, contract);
  assert.equal(report.holdout.length, 4); assert.equal(report.anchor.length, 4);
  let wins, node;
  if (suite.id === 'holdout') wins = report.holdout;
  else {
    const r = spawnSync(process.execPath, ['--test', '--test-reporter=tap', 'functions/index.test.js'], {
      cwd: root, encoding: 'utf8', timeout: 30000,
    });
    assert.equal(r.signal, null); assert.equal(r.status, 0);
    assert.match(r.stdout, /# pass 8\b/); assert.match(r.stdout, /# fail 0\b/); assert.match(r.stdout, /# skipped 0\b/);
    const names = r.stdout.split('\n').filter(x => x.startsWith('# Subtest:')).sort();
    assert.equal(names.length, 8);
    if (root === baseline) retainedNames = names;
    assert.deepEqual(names, retainedNames);
    node = { exit: r.status, stdout: r.stdout, names };
    wins = [...report.anchor, true];
  }
  const primary = wins.filter(Boolean).length / wins.length;
  const score = { primary, noopRate: 1 - primary, costPerWin: 0, regressed: suite.id === 'anchor' && wins.some(x => !x), itemWins: wins };
  measurements.push({ source: policy.retention, suite: suite.id, report, node, score });
  assert.equal(sourceHash(root), policy.retention);
  return score;
}
const fingerprint = fw.gateFingerprint(fw.meetsPromotionRule);
const result = await fw.runFlywheelGenerations({
  rootPolicy: { retention: parentHash },
  proposer: async () => ({ value: candidateHash, summary: 'Apply retention to the existing error summary instead of a nonexistent cost view', inverse: { path: source, parentBytes: readFileSync(join(baseline, source), 'utf8'), parentSha256: parentHash } }),
  evaluator, holdout: { id: 'holdout', items: contract.holdout },
  anchor: { id: 'anchor', items: [...contract.anchor, 'eight original live HTTP tests retained'] },
  maxGenerations: 1, signer: fw.makeSigner(), promotionRule: fw.meetsPromotionRule,
  dataSource: 'LIVE_POSTGRES_TIMESCALE_MIGRATIONS_CATALOG_QUERIES_AND_HTTP_TESTS',
});
const replay = fw.verifyReplayBundle(result.replayBundle, { pinnedGateFingerprint: fingerprint });
assert.equal(replay.pass, true); assert.equal(result.promotions.length, 1);
const evidence = {
  contract, contractSha256: hash(contractBytes), probeSha256: hash(readFileSync(new URL('./probe-migrations.mjs', import.meta.url))),
  parentHash, candidateHash, measurements, promotions: result.promotions.length,
  fingerprint, replay, replayBundle: result.replayBundle,
  boundary: 'Component migration contract only, not whole-repository approval. Actual PostgreSQL/TimescaleDB execution, no database mocks. Every evaluation creates and removes an isolated disposable database. No deployed schema or production persistence changes executed. Source rollback bytes retained; reverting source cannot restore expired data or reverse a deployed migration. Cost/latency/energy unmeasured: costPerWin=0 is a neutral gate input, not a measurement.',
};
writeFileSync(output, JSON.stringify(evidence, null, 2) + '\n');
console.log(JSON.stringify({ promotions: evidence.promotions, replay, measurements: measurements.map(x => ({ suite: x.suite, primary: x.score.primary, counts: x.report.counts, cleanup: x.report.cleanup, nodeExit: x.node?.exit })) }));

const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const { test } = require('node:test');
const { resolve } = require('node:path');

for (const fixture of [
  { name: 'requires explicit ephemeral opt-in', opt: '', url: 'postgres://test_user:fixture@127.0.0.1/observatory_migration_ci', message: /explicit ephemeral fixture opt-in required/ },
  { name: 'rejects remote database endpoints', opt: '1', url: 'postgres://test_user:fixture@example.invalid/observatory_migration_ci', message: /remote\/deployed databases are forbidden/ },
  { name: 'rejects other loopback databases', opt: '1', url: 'postgres://test_user:fixture@127.0.0.1/production', message: /only dedicated test fixture is permitted/ },
]) {
  test(`migration evaluator ${fixture.name}`, () => {
    const r = spawnSync(process.execPath, ['scripts/probe-migrations.mjs', '.'], {
      cwd: resolve(__dirname, '..'), encoding: 'utf8', timeout: 5000,
      env: { ...process.env, METAHARNESS_EPHEMERAL_POSTGRES: fixture.opt, MIGRATION_TEST_DATABASE_URL: fixture.url },
    });
    assert.equal(r.signal, null); assert.equal(r.status, 1);
    assert.match(r.stderr, fixture.message);
    assert.doesNotMatch(r.stderr, /psql:|CREATE DATABASE|DROP DATABASE/);
  });
}

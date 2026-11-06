import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { randomBytes, createHash } from 'node:crypto';
import { resolve, join } from 'node:path';
import { pathToFileURL } from 'node:url';

export const hash = value => createHash('sha256').update(value).digest('hex');
export function probeMigrations(root, contract) {
  assert.equal(process.env.METAHARNESS_EPHEMERAL_POSTGRES, '1', 'explicit ephemeral fixture opt-in required');
  const admin = new URL(process.env.MIGRATION_TEST_DATABASE_URL);
  assert(['postgres:', 'postgresql:'].includes(admin.protocol));
  assert(['127.0.0.1', 'localhost', '[::1]'].includes(admin.hostname), 'remote/deployed databases are forbidden');
  assert.equal(admin.pathname, '/observatory_migration_ci', 'only dedicated test fixture is permitted');
  const db = `mh_observatory_${process.pid}_${randomBytes(6).toString('hex')}`;
  assert(/^mh_observatory_[0-9]+_[a-f0-9]{12}$/.test(db));
  const connection = new URL(admin); connection.pathname = '/' + db;
  const commands = [], migrations = [];
  let created = false;
  function sql(url, args, input) {
    const r = spawnSync('psql', ['-X', '-A', '-t', '-v', 'ON_ERROR_STOP=1', '--dbname', url.href, ...args], {
      input, encoding: 'utf8', timeout: 60000, maxBuffer: 8 * 1024 * 1024,
      env: { ...process.env, PGCONNECT_TIMEOUT: '5', PGOPTIONS: '-c statement_timeout=45000' },
    });
    assert.equal(r.signal, null, 'timeout/signal is not a completed SQL execution');
    assert.notEqual(r.status, null, 'psql must execute');
    const result = { exit: r.status, stdout: r.stdout.trim(), stderr: r.stderr.trim() };
    commands.push({ command: ['psql', '-X', 'ON_ERROR_STOP=1', ...args], ...result });
    return result;
  }
  const query = text => {
    const r = sql(connection, ['-c', text]);
    assert.equal(r.exit, 0, r.stderr);
    return r.stdout;
  };
  let report;
  try {
    const create = sql(admin, ['-c', `CREATE DATABASE "${db}"`]);
    assert.equal(create.exit, 0, create.stderr); created = true;
    const dir = join(resolve(root), 'crates/storage/migrations');
    const names = readdirSync(dir).filter(x => /^\d{3}_.*\.sql$/.test(x)).sort();
    assert.equal(names.length, 8, 'all eight production migrations must execute');
    for (const name of names) {
      const file = join(dir, name);
      const r = sql(connection, ['-f', file]);
      migrations.push({ file: name, sha256: hash(readFileSync(file)), ...r });
      if (r.exit !== 0) break;
      if (name.startsWith('004_')) {
        const r = sql(connection, ['-c', "INSERT INTO llm_traces(ts,trace_id,span_id,span_name,provider,model,input_type,input_text,duration_ms,status_code,total_tokens,total_cost_usd) VALUES (CURRENT_TIMESTAMP,'mh-preserved','mh-span','fixture','test','synthetic','text','preserve this trace',11,'OK',7,0.125)"]);
        assert.equal(r.exit, 0, r.stderr);
      }
    }
    const versions = query("SELECT json_build_object('postgres',current_setting('server_version'),'timescale',(SELECT extversion FROM pg_extension WHERE extname='timescaledb'))");
    const raw = Number(query("SELECT count(*) FROM timescaledb_information.hypertables WHERE hypertable_schema='public' AND hypertable_name IN ('llm_traces','llm_metrics','llm_logs')"));
    const aggregates = Number(query("SELECT count(*) FROM timescaledb_information.continuous_aggregates WHERE view_schema='public' AND view_name IN ('llm_metrics_1min','llm_metrics_1hour','llm_metrics_1day','llm_error_summary')"));
    const preserved = Number(query("SELECT count(*) FROM llm_traces WHERE trace_id='mh-preserved' AND span_id='mh-span' AND input_text='preserve this trace' AND duration_ms=11 AND total_tokens=7 AND total_cost_usd=0.125"));
    // Map continuous-aggregate materialization tables back to their public views.
    const policies = JSON.parse(query("SELECT coalesce(json_agg(x),'[]'::json) FROM (SELECT coalesce(ca.view_name,j.hypertable_name) AS target,j.proc_name,j.config FROM timescaledb_information.jobs j LEFT JOIN timescaledb_information.continuous_aggregates ca ON ca.materialization_hypertable_name=j.hypertable_name AND ca.materialization_hypertable_schema=j.hypertable_schema WHERE j.proc_name IN ('policy_retention','policy_compression') ORDER BY target,j.proc_name) x"));
    const expected = Object.entries(contract.retentionDays).map(([name, days]) => `('${name}',INTERVAL '${days} days')`).join(',');
    const retention = Number(query(`WITH expected(target,age) AS (VALUES ${expected}), actual AS (SELECT coalesce(ca.view_name,j.hypertable_name) AS target,(j.config->>'drop_after')::interval AS age FROM timescaledb_information.jobs j LEFT JOIN timescaledb_information.continuous_aggregates ca ON ca.materialization_hypertable_name=j.hypertable_name AND ca.materialization_hypertable_schema=j.hypertable_schema WHERE j.proc_name='policy_retention') SELECT count(*) FROM expected JOIN actual USING(target,age)`));
    const compression = Number(query(`SELECT count(*) FROM timescaledb_information.jobs WHERE proc_name='policy_compression' AND hypertable_schema='public' AND hypertable_name IN ('llm_traces','llm_metrics','llm_logs') AND (config->>'compress_after')::interval=INTERVAL '${contract.compressionDays} days'`));
    const applied005 = migrations.some(x => x.file.startsWith('005_') && x.exit === 0);
    report = {
      root: resolve(root), versions: JSON.parse(versions), migrations, policies,
      holdout: [applied005, retention === 7 && policies.filter(x => x.proc_name === 'policy_retention').length === 7, compression === 3, migrations.length === 8 && migrations.every(x => x.exit === 0)],
      anchor: [migrations.filter(x => /^00[1-4]_/.test(x.file)).length === 4 && migrations.slice(0,4).every(x => x.exit === 0), raw === 3, aggregates === 4, preserved === 1],
      counts: { retention, compression, raw, aggregates, preserved }, commands,
    };
    return report;
  } finally {
    if (created) {
      const r = sql(admin, ['-c', `DROP DATABASE "${db}" WITH (FORCE)`]);
      assert.equal(r.exit, 0, 'owned ephemeral database cleanup must succeed: ' + r.stderr);
      if (report) report.cleanup = { database: db, exit: r.exit, disposableFixtureOnly: true };
    }
  }
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const root = resolve(process.argv[2] || '.');
  const contract = JSON.parse(readFileSync(new URL('../docs/metaharness/2026-10-18/contract.json', import.meta.url)));
  const r = probeMigrations(root, contract);
  console.log(JSON.stringify(r, null, 2));
  process.exitCode = [...r.holdout, ...r.anchor].every(Boolean) ? 0 : 1;
}

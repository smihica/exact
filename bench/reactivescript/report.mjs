// Summarizes a round that run.sh made: per desk, the defects as the harness's round 11 counts them (failed hidden
// tests, failed overlap tests, and fuzz seeds that found a hole), the agent's ./check and ./test calls, its turns and
// cost; then the mean defects and cost per arm and tier.
// Usage: node report.mjs <round dir>
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import path from 'node:path';

const round = path.resolve(process.argv[2]);
const read = (d, f) => (existsSync(path.join(d, f)) ? readFileSync(path.join(d, f), 'utf8') : '');

function section(grade, name) {
  const m = grade.match(new RegExp(`== ${name}\\n([\\s\\S]*?)(?=\\n== |$)`));
  return m ? m[1] : '';
}

const rows = [];
for (const name of readdirSync(round).sort()) {
  const m = name.match(/^desk9-(\w+)-(\d+)$/);
  if (!m) continue;
  const d = path.join(round, name), grade = read(d, 'grade.txt');
  const failed = (s) => s.split('\n').filter((l) => l.startsWith('✗')).length;
  const hidden = failed(section(grade, 'hidden')), overlap = failed(section(grade, 'overlap'));
  const fuzz = Number(section(grade, 'fuzz').match(/(\d+)\/\d+ seeds found a hole/)?.[1] ?? NaN);
  let agent = {};
  try { agent = JSON.parse(read(d, 'agent.json')); } catch {}
  const log = read(d, '.runlog').split('\n').filter(Boolean).map((l) => JSON.parse(l));
  rows.push({
    desk: name, arm: m[1], tier: read(d, 'tier').trim(), hidden, overlap, fuzz,
    defects: hidden + overlap + fuzz, calls: log.length, turns: agent.num_turns ?? null, cost: agent.total_cost_usd ?? null,
    built: !/build failed/.test(grade),
  });
}

const fmt = (v, digits = 1) => (v == null || Number.isNaN(v) ? '—' : typeof v === 'number' ? v.toFixed(digits) : v);
console.log(`# Round ${path.basename(round)}\n\n${read(round, 'round.txt').trim()}\n`);
console.log('| desk | tier | hidden ✗ | overlap ✗ | fuzz seeds | defects | ./check+./test | turns | cost |');
console.log('|---|---|---:|---:|---:|---:|---:|---:|---:|');
for (const r of rows) console.log(`| ${r.desk} | ${r.tier} | ${r.hidden} | ${r.overlap} | ${r.fuzz} | ${r.defects}${r.built ? '' : ' (no build)'} | ${r.calls} | ${fmt(r.turns, 0)} | ${r.cost == null ? '—' : `$${r.cost.toFixed(2)}`} |`);

console.log('\n| arm | tier | desks | mean defects | correct apps | mean cost |');
console.log('|---|---|---:|---:|---:|---:|');
const mean = (xs) => (xs.length ? xs.reduce((a, b) => a + b, 0) / xs.length : NaN);
for (const arm of [...new Set(rows.map((r) => r.arm))]) for (const tier of ['opus', 'haiku']) {
  const rs = rows.filter((r) => r.arm === arm && r.tier === tier);
  if (!rs.length) continue;
  const costs = rs.map((r) => r.cost).filter((c) => c != null);
  console.log(`| ${arm} | ${tier} | ${rs.length} | ${fmt(mean(rs.map((r) => r.defects)))} | ${rs.filter((r) => r.defects === 0).length} | ${costs.length ? `$${mean(costs).toFixed(2)}` : '—'} |`);
}

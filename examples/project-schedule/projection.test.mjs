import test from 'node:test';
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {readFileSync, mkdtempSync, writeFileSync, rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {project, bounds, validDate, shiftDate, ruleSource, displayDate} from './projection.mjs';
function fixture(operator, reversed = false) {
  const fields = ['start_date','end_date'].map(local_id => ({id:local_id,local_id,name:local_id,required:true,value_type:{kind:'date'}}));
  return {module:{models:[{id:'project',fields}],constraints:[{id:'rule',model_id:'project',left:{kind:'field',value:reversed?'start_date':'end_date'},right:{kind:'field',value:reversed?'end_date':'start_date'},operator}]},diagnostics:[]};
}
for (const [operator, sign, strict] of [['greater_than',1,true],['greater_than_or_equal',1,false],['less_than',-1,true],['less_than_or_equal',-1,false]]) {
  for (const reversed of [false,true]) test(`${operator}, reversed=${reversed}: both directions and equality`, () => {
    const p = project(fixture(operator,reversed));
    const [left,right] = reversed ? ['start_date','end_date'] : ['end_date','start_date'];
    const b = bounds(p,{[left]:'2024-03-01',[right]:'2024-03-01'});
    assert.equal(b[left][sign>0?'min':'max'], strict?shiftDate('2024-03-01',sign):'2024-03-01');
    assert.equal(b[right][sign>0?'max':'min'], strict?shiftDate('2024-03-01',-sign):'2024-03-01');
  });
}
test('calendar boundaries, leap years, invalid input and range', () => {
  assert.equal(shiftDate('2024-03-01',-1),'2024-02-29');
  assert.equal(shiftDate('2025-01-01',-1),'2024-12-31');
  assert.equal(shiftDate('0099-12-31',1),'0100-01-01');
  assert.equal(shiftDate('0001-01-01',-1),null);
  assert.equal(shiftDate('9999-12-31',1),null);
  for (const value of ['', '2023-02-29','1900-02-29','2024-04-31','0000-01-01','10000-01-01','2024-1-01']) assert.equal(validDate(value),false,value);
  assert.equal(validDate('2000-02-29'),true);
  for (const timezone of ['UTC','America/Los_Angeles','Asia/Seoul']) {
    const old = process.env.TZ; process.env.TZ = timezone;
    assert.equal(shiftDate('2024-03-10',1),'2024-03-11'); process.env.TZ = old;
  }
});
test('empty/invalid peers remove stale bounds without changing values', () => {
  const p=project(fixture('greater_than'));
  assert.equal(bounds(p,{start_date:'2024-01-01'}).end_date.min,'2024-01-02');
  assert.deepEqual(bounds(p,{start_date:'',end_date:''}),{start_date:{},end_date:{}});
  assert.deepEqual(bounds(p,{start_date:'2024-02-30'}).end_date,{});
  assert.equal(bounds(p,{start_date:'9999-12-31'}).end_date.unavailable,true);
});
test('unsupported and failed compilations are explicit', () => {
  assert.throws(()=>project({module:null,diagnostics:[]}));
  assert.throws(()=>project(fixture('equal')));
  const f=fixture('greater_than'); f.diagnostics=[{severity:'error'}]; assert.throws(()=>project(f));
});
test('UTF-8 byte source spans preserve Korean rule evidence', () => {
  const source='앞말\n마감일은 시작일보다 커야 한다.\n';
  const start=new TextEncoder().encode('앞말\n').length;
  assert.equal(ruleSource(source,{start,end:new TextEncoder().encode(source).length-1}),'마감일은 시작일보다 커야 한다.');
});
const cli=process.env.RSPDL_BIN ?? fileURLToPath(new URL('../../target/debug/rspdl',import.meta.url));
const sourcePath=fileURLToPath(new URL('./project.rspdl',import.meta.url));
test('actual single-file CLI canonical IR', () => {
  const compilation=JSON.parse(execFileSync(cli,['compile',sourcePath,'--json'],{encoding:'utf8'}));
  const p=project(compilation);
  assert.equal(p.constraints[0].operator,'greater_than');
  const source=readFileSync(sourcePath,'utf8');
  assert.match(ruleSource(source,p.constraints[0].span),/마감일/);
  const start=p.fields.find(f=>f.local_id==='start_date').id;
  const end=p.fields.find(f=>f.local_id==='end_date').id;
  assert.equal(bounds(p,{[start]:'2024-02-28'})[end].min,'2024-02-29');
});

test('actual CLI check violation exposes typed dates for readable evidence', () => {
  const directory = mkdtempSync(join(tmpdir(), 'rspdl-projection-'));
  try {
    const data = join(directory, 'data.json');
    writeFileSync(data, JSON.stringify({records:{'project_schedule.project':[
      {$id:'project-1',start_date:'2026-10-06',end_date:'2026-10-05'},
    ]}}));
    let output;
    try { output = execFileSync(cli,['check',sourcePath,'--data',data,'--json'],{encoding:'utf8'}); }
    catch (error) { assert.equal(error.status, 2); output = error.stdout; }
    const report = JSON.parse(output);
    assert.equal(report.runtime_diagnostics.length,0);
    assert.equal(report.constraint_violations.length,1);
    const violation=report.constraint_violations[0];
    assert.equal(displayDate(violation.left),'2026-10-05');
    assert.equal(displayDate(violation.right),'2026-10-06');
    assert.equal(project(report.compilation).constraints[0].id,violation.constraint_id);
    assert.throws(()=>displayDate({value:'2026-10-05'}));
  } finally { rmSync(directory,{recursive:true,force:true}); }
});

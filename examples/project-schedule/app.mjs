import { project, bounds, operators, ruleSource, displayDate } from './projection.mjs';
const form = document.querySelector('#schedule');
const save = document.querySelector('#save');
const status = document.querySelector('#status');
const evidence = document.querySelector('#evidence');
let schema, projection, revision = 0, timer, saving = false;
const controls = new Map();
/** Read the current date strings keyed by canonical field ID. */
const values = () => Object.fromEntries([...controls].map(([id, control]) => [id, control.input.value]));
/** Clear field messages before displaying evidence for the current revision. */
function clearErrors() {
  for (const c of controls.values()) { c.error.textContent = ''; c.input.removeAttribute('aria-invalid'); }
}
/** Attach a compiler-derived message to a known projected field. */
function fieldError(id, text) {
  const c = controls.get(id);
  if (c) { c.error.textContent += `${text} `; c.input.setAttribute('aria-invalid', 'true'); }
}
/** Fetch same-origin schema or send a JSON candidate to the local API. */
async function request(url, body) {
  const response = await fetch(url, body === undefined ? {} : { method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify(body) });
  if (!response.ok) throw new Error(`서버 요청 실패 (${response.status})`);
  return response.json();
}
/** Render CLI evidence and reject inconsistent acceptance or save claims. */
function renderReport(result) {
  const report = result.report;
  if (!report?.compilation || !Array.isArray(report.runtime_diagnostics) || !Array.isArray(report.constraint_violations) || typeof result.accepted !== 'boolean' || typeof result.saved !== 'boolean') throw new Error('검증 응답 형식이 올바르지 않습니다.');
  clearErrors();
  evidence.textContent = JSON.stringify(report, null, 2);
  const messages = [];
  for (const diagnostic of report.compilation.diagnostics) messages.push(`기획 진단: ${diagnostic.message ?? diagnostic.message_key ?? diagnostic.rule_id}`);
  for (const diagnostic of report.runtime_diagnostics) {
    const field = projection.fields.find(f => f.local_id === diagnostic.arguments?.field_id || f.id === diagnostic.arguments?.field_id || diagnostic.path?.endsWith(`.${f.local_id}`));
    const message = diagnostic.message_key === 'runtime.field.required_missing' ? `${field?.name ?? '필수 날짜'}을 입력하세요.` : diagnostic.message_key === 'runtime.value.type_mismatch' ? `${field?.name ?? '날짜'}에 올바른 날짜를 입력하세요. (${diagnostic.rule_id})` : `${field?.name ?? '입력값'}을 확인하세요. ${diagnostic.message_key} (${diagnostic.rule_id})`;
    messages.push(message);
    if (field) fieldError(field.id, message);
  }
  for (const violation of report.constraint_violations) {
    const rule = projection.constraints.find(c => c.id === violation.constraint_id);
    if (!rule) { messages.push(`지원하지 않는 규칙 위반: ${violation.constraint_id}`); continue; }
    const left = projection.fields.find(f => f.id === rule.left.value);
    const right = projection.fields.find(f => f.id === rule.right.value);
    const message = `${left.name}은 ${right.name}${operators[rule.operator].label}. 입력: ${left.name} ${displayDate(violation.left)}, ${right.name} ${displayDate(violation.right)}. 근거: ${ruleSource(schema.source, rule.span)}`;
    messages.push(message);
    fieldError(left.id, message); fieldError(right.id, message);
  }
  const hasErrors = report.compilation.diagnostics.some(d => d.severity === 'error') || report.runtime_diagnostics.some(d => d.severity === 'error') || report.constraint_violations.length > 0 || report.policy_results?.some(p => p.status !== 'allowed');
  if (result.accepted && hasErrors || result.saved && !result.accepted) throw new Error('검증 결과가 일치하지 않습니다.');
  status.textContent = result.saved ? '프로젝트 일정이 저장되었습니다.' : result.accepted ? '입력한 일정이 규칙을 충족합니다. 저장할 수 있습니다.' : messages.join('\n') || '검증을 통과하지 못했습니다. 입력값을 확인하세요.';
}
/** Check current fields and display the response only while its revision is current. */
async function validate(endpoint, current) {
  try {
    const record = Object.fromEntries(projection.fields.filter(f => controls.get(f.id).input.value !== '').map(f => [f.local_id, controls.get(f.id).input.value]));
    const result = await request(endpoint, {record});
    if (current === revision) renderReport(result);
  } catch (error) {
    if (current === revision) { clearErrors(); status.textContent = `검증할 수 없습니다. 저장 완료로 처리하지 않았습니다. ${error.message}`; }
  }
}
/** Refresh projected date bounds and debounce authoritative CLI validation. */
function changed() {
  revision++; clearTimeout(timer); clearErrors();
  const projected = bounds(projection, values());
  for (const [id, c] of controls) {
    const bound = projected[id];
    for (const key of ['min','max']) bound[key] ? c.input.setAttribute(key,bound[key]) : c.input.removeAttribute(key);
    c.hint.textContent = bound.unavailable ? '선택 가능한 날짜 범위가 없습니다. 상대 날짜를 확인하세요.' : [bound.min && `${bound.min} 이후`, bound.max && `${bound.max} 이전`].filter(Boolean).join(' · ');
  }
  status.textContent = '입력한 일정을 확인하는 중입니다.';
  const current = revision;
  timer = setTimeout(() => validate('/api/check', current), 200);
}
form.addEventListener('submit', async event => {
  event.preventDefault();
  if (!projection || saving) return;
  clearTimeout(timer); const current = ++revision;
  saving = true; save.disabled = true; status.textContent = '일정을 검증하고 저장하는 중입니다.';
  // novalidate deliberately sends even empty/out-of-range inputs to compiler validation.
  await validate('/api/save', current);
  saving = false; save.disabled = false;
});
try {
  schema = await request('/api/schema');
  document.querySelector('#source').textContent = schema.source;
  evidence.textContent = JSON.stringify(schema.compilation, null, 2);
  projection = project(schema.compilation);
  for (const field of projection.fields) {
    const prefix = field.local_id === 'start_date' ? 'start' : 'end';
    const input = document.getElementById(field.local_id);
    document.getElementById(`${prefix}-label`).textContent = field.name;
    controls.set(field.id, {input, hint:document.getElementById(`${prefix}-hint`), error:document.getElementById(`${prefix}-error`)});
    input.addEventListener('input', changed);
  }
  status.textContent = '날짜를 입력하면 기획 규칙을 확인합니다.'; save.disabled = false;
} catch (error) {
  projection = null; save.disabled = true;
  const diagnostics = schema?.compilation?.diagnostics ?? [];
  status.textContent = [`일정 등록을 시작할 수 없습니다. ${error.message}`, ...diagnostics.map(d => `${d.rule_id}: ${d.message_key} ${JSON.stringify(d.arguments ?? {})}`)].join('\n');
  for (const input of form.querySelectorAll('input')) input.disabled = true;
}

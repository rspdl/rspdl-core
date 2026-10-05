// Application projection of public Canonical IR. Bounds are hints; the server checks rules.
export const operators = {
  less_than: { direction: -1, strict: true, label: '보다 앞서야 합니다' },
  less_than_or_equal: { direction: -1, strict: false, label: '보다 앞서거나 같아야 합니다' },
  greater_than: { direction: 1, strict: true, label: '보다 뒤여야 합니다' },
  greater_than_or_equal: { direction: 1, strict: false, label: '보다 뒤이거나 같아야 합니다' },
};
/** Accepts Gregorian YYYY-MM-DD dates in years 0001–9999, including leap-day checks. */
export function validDate(value) {
  if (typeof value !== 'string' || !/^\d{4}-\d{2}-\d{2}$/.test(value)) return false;
  const [year, month, day] = value.split('-').map(Number);
  if (year < 1 || month < 1 || month > 12 || day < 1) return false;
  const leap = year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
  return day <= [31, leap ? 29 : 28,31,30,31,30,31,31,30,31,30,31][month-1];
}
/** Shifts a valid date by the caller-supplied integer count of UTC days; returns null for invalid input or year overflow. */
export function shiftDate(value, days) {
  if (!validDate(value)) return null;
  const date = new Date(`${value}T00:00:00.000Z`);
  date.setUTCDate(date.getUTCDate() + days);
  const year = date.getUTCFullYear();
  return year >= 1 && year <= 9999 ? date.toISOString().slice(0,10) : null;
}
/** Extracts an IR half-open UTF-8 byte span; throws if its slice splits a code point. */
export function ruleSource(source, span) {
  return new TextDecoder('utf-8', { fatal: true }).decode(new TextEncoder().encode(source).slice(span.start, span.end));
}
/**
 * Accepts error-free IR for one model with two required date fields and supported
 * field-to-field ordering rules. Throws when compilation or this example contract fails.
 */
export function project(compilation) {
  if (!compilation?.module || !Array.isArray(compilation.diagnostics) || compilation.diagnostics.some(d => d.severity === 'error')) throw new Error('기획 컴파일에 실패했습니다.');
  const module = compilation.module;
  if (module.models?.length !== 1) throw new Error('이 예제는 프로젝트 모델 하나만 지원합니다.');
  const model = module.models[0];
  const fields = model.fields;
  if (fields?.length !== 2 || !['start_date','end_date'].every(id => fields.some(f => f.local_id === id && f.required && f.value_type?.kind === 'date'))) throw new Error('이 예제는 필수 시작일·마감일 날짜 필드만 지원합니다.');
  const ids = new Set(fields.map(f => f.id));
  if (!module.constraints?.length || module.constraints.some(c => c.model_id !== model.id || c.left?.kind !== 'field' || c.right?.kind !== 'field' || !ids.has(c.left.value) || !ids.has(c.right.value) || c.left.value === c.right.value || !operators[c.operator])) throw new Error('지원하지 않는 날짜 규칙 projection입니다.');
  return { model, fields, constraints: module.constraints };
}
/**
 * Intersects per-field min/max hints from valid peer dates, shifting strict bounds by
 * one UTC day. Invalid peers are ignored; year overflow marks a field unavailable.
 * Requires a validated projection; server validation still decides rule satisfaction.
 */
export function bounds(projection, values) {
  const result = Object.fromEntries(projection.fields.map(f => [f.id, {}]));
  for (const rule of projection.constraints) {
    const {direction, strict} = operators[rule.operator];
    for (const [own, peer, sign] of [[rule.left.value,rule.right.value,direction],[rule.right.value,rule.left.value,-direction]]) {
      if (!validDate(values[peer])) continue;
      const limit = shiftDate(values[peer], strict ? sign : 0);
      if (limit === null) { result[own].unavailable = true; continue; }
      const key = sign > 0 ? 'min' : 'max';
      const previous = result[own][key];
      if (!previous || (key === 'min' ? limit > previous : limit < previous)) result[own][key] = limit;
    }
  }
  return result;
}

/** Returns a valid canonical date representation; throws for unsupported value shapes. */
export function displayDate(value) {
  if (value?.value_type?.kind === 'date' && value?.representation?.kind === 'date' && validDate(value.representation.value)) return value.representation.value;
  throw new Error('지원하지 않는 날짜 검증 값 형식입니다.');
}

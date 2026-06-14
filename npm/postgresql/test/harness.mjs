// Replay harness for upstream eslint-plugin-postgresql fixture cases.
//
// Each case's `code` (raw SQL) is run through the ported rule's `createOnce`/
// `Program` adapter — the same path the Oxlint runtime drives — and the reported
// diagnostics are formatted into ESLint's reported-error shape for comparison
// against the case's declared `errors`. Cases that declare `output` have all
// reported fixes applied in a single ESLint-compatible pass and compared.
//
// The port's descriptor `loc` carries 1-indexed lines and 0-indexed columns
// (the parser counts UTF-16 columns from 0, matching ESLint's `loc`); the
// fixtures store RuleTester's 1-indexed `column`/`endColumn`, so the harness
// adds 1 to every column. Rule options are passed through untouched — the real
// adapter maps them onto the native scan options, so the harness exercises the
// shipping option-handling code.

import plugin from '../index.js';

function interpolate(template, data) {
  if (!data || template == null) {
    return template;
  }
  return template.replace(/\{\{\s*([^}\s]+)\s*\}\}/g, (match, key) =>
    Object.prototype.hasOwnProperty.call(data, key) ? String(data[key]) : match,
  );
}

// Turn a `context.report` descriptor into ESLint's reported error shape.
function formatReport(rule, descriptor) {
  const messages = rule.meta.messages ?? {};
  const message = descriptor.messageId
    ? interpolate(messages[descriptor.messageId], descriptor.data)
    : descriptor.message;

  const result = { message };
  if (descriptor.messageId) {
    result.messageId = descriptor.messageId;
  }

  const loc = descriptor.loc;
  if (loc) {
    if (loc.start) {
      result.line = loc.start.line;
      result.column = loc.start.column + 1;
    }
    if (loc.end) {
      result.endLine = loc.end.line;
      result.endColumn = loc.end.column + 1;
    }
  }

  return result;
}

// Extract a `{ range, text }` fix from a report descriptor, if any.
function extractFix(descriptor) {
  if (typeof descriptor.fix !== 'function') {
    return null;
  }
  const fix = descriptor.fix({
    replaceTextRange(range, text) {
      return { range, text };
    },
  });
  return fix && Array.isArray(fix.range) ? fix : null;
}

// Apply all reported fixes in a single ESLint-compatible pass: sort by range,
// skip any fix overlapping an already-applied one (matching SourceCodeFixer).
function applyFixes(code, descriptors) {
  const fixes = descriptors
    .map(extractFix)
    .filter(Boolean)
    .sort((a, b) => a.range[0] - b.range[0] || a.range[1] - b.range[1]);

  let output = '';
  let cursor = 0;
  for (const fix of fixes) {
    const [start, end] = fix.range;
    if (start < cursor) {
      continue;
    }
    output += code.slice(cursor, start) + fix.text;
    cursor = end;
  }
  output += code.slice(cursor);
  return output;
}

// Run a rule over one case and return both the formatted reports and the raw
// descriptors (the latter needed to apply fixes for `output` assertions).
export function runRule(ruleName, testCase) {
  const rule = plugin.rules[ruleName];
  if (!rule) {
    throw new Error(`Unknown rule: ${ruleName}`);
  }

  // Upstream's RuleTester lints `readSql(...).trim()`, so the captured fixture
  // `code` (which keeps its trailing newline) must be trimmed to reproduce the
  // expected error columns and autofix `output` exactly.
  const code = testCase.code.trim();
  const sourceCode = {
    text: code,
    getText() {
      return code;
    },
  };

  const descriptors = [];
  const context = {
    id: ruleName,
    options: testCase.options ?? [],
    sourceCode,
    filename: testCase.filename ?? 'file.sql',
    report(descriptor) {
      descriptors.push(descriptor);
    },
  };

  const visitor = rule.createOnce(context);
  visitor.Program?.({ type: 'Program', range: [0, code.length] });

  return {
    reports: descriptors.map((descriptor) => formatReport(rule, descriptor)),
    output: applyFixes(code, descriptors),
  };
}

// Assert one reported error matches one declared expectation. A string
// expectation checks only the message; an object checks each declared field
// (ignoring `type`/`suggestions`, which the port does not model).
export function matchError(actual, expected) {
  if (typeof expected === 'string') {
    return { ok: actual.message === expected, field: 'message', actualValue: actual.message };
  }

  for (const key of Object.keys(expected)) {
    if (key === 'type' || key === 'suggestions' || key === 'data') {
      continue;
    }
    if (actual[key] !== expected[key]) {
      return { ok: false, field: key, actualValue: actual[key] };
    }
  }

  return { ok: true, field: null };
}

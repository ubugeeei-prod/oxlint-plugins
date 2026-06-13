// Replay harness for upstream eslint-plugin-security RuleTester cases.
//
// Unlike a typical ESLint plugin, security's rule logic runs entirely in Rust
// (`scanSecurity`); the JS layer is only an Oxlint/NAPI adapter. So each case's
// `code` is fed to the plugin's `createOnce` adapter (which calls the Rust
// scanner and filters to the rule under test), and the reported descriptors are
// formatted into ESLint's reported shape for comparison against the case's
// declared `errors`.

import plugin from '../index.js';

// Drive one rule's adapter over `code` and return ESLint-shaped reports.
export function runRule(ruleName, testCase) {
  const rule = plugin.rules[ruleName];
  if (!rule) {
    throw new Error(`Unknown rule: ${ruleName}`);
  }

  const code = testCase.code;
  const sourceCode = {
    text: code,
    getText() {
      return this.text;
    },
  };

  const reports = [];
  const context = {
    id: ruleName,
    options: testCase.options ?? [],
    sourceCode,
    filename: testCase.filename ?? 'file.js',
    report(descriptor) {
      reports.push(descriptor);
    },
  };

  const visitor = rule.createOnce(context);
  visitor.before?.();
  visitor.Program?.({ type: 'Program', range: [0, code.length] });
  visitor.after?.();

  return reports.map((descriptor) => formatReport(rule, descriptor));
}

function interpolate(template, data) {
  if (!data) {
    return template;
  }
  return template.replace(/\{\{\s*([^}\s]+)\s*\}\}/g, (match, key) =>
    Object.prototype.hasOwnProperty.call(data, key) ? String(data[key]) : match,
  );
}

// Turn a `context.report` descriptor into ESLint's reported error shape.
// ESLint reports 1-indexed columns (`loc.column + 1`); the Rust scanner and the
// adapter both emit 0-indexed UTF-16 columns, matching ESLint's internal `loc`.
function formatReport(rule, descriptor) {
  const messages = rule.meta.messages ?? {};
  const message = descriptor.messageId
    ? interpolate(messages[descriptor.messageId], descriptor.data)
    : descriptor.message;

  const result = { message };
  if (descriptor.messageId) {
    result.messageId = descriptor.messageId;
  }
  if (descriptor.data) {
    result.data = descriptor.data;
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

// Assert one reported error matches one declared expectation. A string
// expectation checks only the message; an object checks each declared field.
// `message` may be a literal string or a `{ __regex, __flags }` matcher.
export function matchError(actual, expected) {
  if (typeof expected === 'string') {
    return { ok: actual.message === expected, field: 'message' };
  }

  for (const key of Object.keys(expected)) {
    if (key === 'type' || key === 'suggestions') {
      continue;
    }
    if (key === 'message') {
      if (!messageMatches(actual.message, expected.message)) {
        return { ok: false, field: 'message' };
      }
      continue;
    }
    if (key === 'data') {
      if (JSON.stringify(actual.data ?? {}) !== JSON.stringify(expected.data)) {
        return { ok: false, field: 'data' };
      }
      continue;
    }
    if (actual[key] !== expected[key]) {
      return { ok: false, field: key };
    }
  }

  return { ok: true, field: null };
}

function messageMatches(actualMessage, expectedMessage) {
  if (expectedMessage && typeof expectedMessage === 'object' && '__regex' in expectedMessage) {
    const regex = new RegExp(expectedMessage.__regex, expectedMessage.__flags ?? '');
    return regex.test(actualMessage ?? '');
  }
  return actualMessage === expectedMessage;
}

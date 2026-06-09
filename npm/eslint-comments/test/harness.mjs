// Replay harness for upstream eslint-comments RuleTester cases.
//
// Each case's `code` is parsed with espree (ESLint's own parser, so comment
// `loc`/`range` match what the real plugin runtime would provide), the rule is
// run through its `createOnce` wrapper, and the reported diagnostics are
// formatted into ESLint's reported shape for comparison against the case's
// declared `errors`.

import * as espree from 'espree';

import plugin from '../index.js';

function parseOptions(testCase) {
  const languageOptions = testCase.languageOptions ?? {};
  const parserOptions = languageOptions.parserOptions ?? testCase.parserOptions ?? {};

  return {
    comment: true,
    loc: true,
    range: true,
    ecmaVersion: languageOptions.ecmaVersion ?? parserOptions.ecmaVersion ?? 'latest',
    sourceType: languageOptions.sourceType ?? parserOptions.sourceType ?? 'script',
    ecmaFeatures: parserOptions.ecmaFeatures,
  };
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
// ESLint reports 1-indexed columns (`loc.column + 1`).
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

export function runRule(ruleName, testCase) {
  const rule = plugin.rules[ruleName];
  if (!rule) {
    throw new Error(`Unknown rule: ${ruleName}`);
  }

  const ast = espree.parse(testCase.code, parseOptions(testCase));
  const comments = ast.comments ?? [];
  const sourceCode = {
    text: testCase.code,
    ast,
    getAllComments: () => comments,
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
  visitor.Program?.(ast);
  visitor.after?.();

  return reports.map((descriptor) => formatReport(rule, descriptor));
}

// Assert one reported error matches one declared expectation. A string
// expectation checks only the message; an object checks each declared field.
export function matchError(actual, expected) {
  if (typeof expected === 'string') {
    return { ok: actual.message === expected, field: 'message' };
  }

  for (const key of Object.keys(expected)) {
    if (key === 'type' || key === 'suggestions') {
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

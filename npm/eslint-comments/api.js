'use strict';

// Low-level, NAPI-backed programmatic API. Each function takes the comments of
// a single file (as produced by an ESLint-compatible `getAllComments()`) and
// returns the diagnostics the matching Oxlint rule would report.

const native = require('@oxlint-plugins/core').eslintComments;

function normalizeComments(comments) {
  if (!Array.isArray(comments)) {
    throw new TypeError('comments must be an array.');
  }

  return comments.map((comment) => {
    if (!comment || typeof comment.value !== 'string') {
      throw new TypeError('each comment must have a string value.');
    }

    return {
      kind: comment.kind === 'Line' ? 'Line' : 'Block',
      value: comment.value,
      startLine: comment.startLine >>> 0,
      startColumn: comment.startColumn | 0,
      endLine: comment.endLine >>> 0,
      endColumn: comment.endColumn | 0,
    };
  });
}

function normalizeStrings(name, values) {
  if (!Array.isArray(values) || values.some((value) => typeof value !== 'string')) {
    throw new TypeError(`${name} must be an array of strings.`);
  }

  return values;
}

function normalizePosition(position) {
  if (!position || typeof position.line !== 'number' || typeof position.column !== 'number') {
    throw new TypeError('position must have numeric line and column fields.');
  }

  return {
    line: position.line >>> 0,
    column: position.column | 0,
  };
}

function normalizeProblems(problems = []) {
  if (!Array.isArray(problems)) {
    throw new TypeError('problems must be an array.');
  }

  return problems.map((problem) => {
    if (!problem || (problem.ruleId != null && typeof problem.ruleId !== 'string')) {
      throw new TypeError('each problem must have a nullable string ruleId.');
    }

    const line = problem.line ?? problem.loc?.start?.line;
    const column = problem.column ?? problem.loc?.start?.column;
    if (typeof line !== 'number' || typeof column !== 'number') {
      throw new TypeError('each problem must have numeric line and column fields.');
    }

    return {
      ruleId: problem.ruleId == null ? null : problem.ruleId,
      line: line >>> 0,
      column: column | 0,
    };
  });
}

function scanDisableEnablePair(comments, allowWholeFile = false, firstTokenStart = null) {
  return native.scanDisableEnablePair(
    normalizeComments(comments),
    !!allowWholeFile,
    firstTokenStart == null ? null : normalizePosition(firstTokenStart),
  );
}

function scanNoAggregatingEnable(comments) {
  return native.scanNoAggregatingEnable(normalizeComments(comments));
}

function scanNoDuplicateDisable(comments) {
  return native.scanNoDuplicateDisable(normalizeComments(comments));
}

function scanNoRestrictedDisable(comments, patterns = []) {
  return native.scanNoRestrictedDisable(
    normalizeComments(comments),
    normalizeStrings('patterns', patterns),
  );
}

function scanNoUnlimitedDisable(comments) {
  return native.scanNoUnlimitedDisable(normalizeComments(comments));
}

function scanNoUnusedDisable(comments, problems = []) {
  return native.scanNoUnusedDisable(normalizeComments(comments), normalizeProblems(problems));
}

function scanNoUnusedEnable(comments) {
  return native.scanNoUnusedEnable(normalizeComments(comments));
}

function scanNoUse(comments, allow = []) {
  return native.scanNoUse(normalizeComments(comments), normalizeStrings('allow', allow));
}

function scanRequireDescription(comments, ignore = []) {
  return native.scanRequireDescription(
    normalizeComments(comments),
    normalizeStrings('ignore', ignore),
  );
}

module.exports = {
  scanDisableEnablePair,
  scanNoAggregatingEnable,
  scanNoDuplicateDisable,
  scanNoRestrictedDisable,
  scanNoUnlimitedDisable,
  scanNoUnusedDisable,
  scanNoUnusedEnable,
  scanNoUse,
  scanRequireDescription,
};
module.exports.default = module.exports;

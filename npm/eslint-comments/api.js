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

function scanNoUnlimitedDisable(comments) {
  return native.scanNoUnlimitedDisable(normalizeComments(comments));
}

module.exports = {
  scanNoUnlimitedDisable,
};
module.exports.default = module.exports;

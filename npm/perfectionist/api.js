'use strict';

const native = require('./native.js');

function scanPerfectionist(sourceText, filename = 'file.tsx') {
  if (typeof sourceText !== 'string') {
    throw new TypeError('sourceText must be a string.');
  }
  if (typeof filename !== 'string') {
    throw new TypeError('filename must be a string.');
  }
  return native.scanPerfectionist(sourceText, filename);
}

function scanPerfectionistRule(
  sourceText,
  filename = 'file.tsx',
  ruleName = 'sort-named-imports',
  options = [],
) {
  if (typeof sourceText !== 'string') {
    throw new TypeError('sourceText must be a string.');
  }
  if (typeof filename !== 'string') {
    throw new TypeError('filename must be a string.');
  }
  if (typeof ruleName !== 'string') {
    throw new TypeError('ruleName must be a string.');
  }
  return native.scanPerfectionistRule(sourceText, filename, ruleName, options);
}

module.exports = {
  implementedPerfectionistRuleNames: native.implementedPerfectionistRuleNames,
  scanPerfectionist,
  scanPerfectionistRule,
};
module.exports.default = module.exports;

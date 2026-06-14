'use strict';

const native = require('./native.js');

function implementedTestingLibraryRuleNames() {
  return native.implementedTestingLibraryRuleNames();
}

function scanTestingLibrary(sourceText, filename = 'file.test.tsx', options = {}) {
  if (typeof sourceText !== 'string') {
    throw new TypeError('sourceText must be a string.');
  }
  if (typeof filename !== 'string') {
    throw new TypeError('filename must be a string.');
  }
  const raw = options && typeof options === 'object' ? options : {};
  return native.scanTestingLibrary(sourceText, filename, {
    ruleNames: Array.isArray(raw.ruleNames)
      ? raw.ruleNames.filter((ruleName) => typeof ruleName === 'string' && ruleName.length > 0)
      : undefined,
    testIdPattern: typeof raw.testIdPattern === 'string' ? raw.testIdPattern : undefined,
    testIdAttribute: Array.isArray(raw.testIdAttribute)
      ? raw.testIdAttribute.filter((value) => typeof value === 'string' && value.length > 0)
      : undefined,
    customMessage: typeof raw.customMessage === 'string' ? raw.customMessage : undefined,
  });
}

module.exports = {
  implementedTestingLibraryRuleNames,
  scanTestingLibrary,
};
module.exports.default = module.exports;

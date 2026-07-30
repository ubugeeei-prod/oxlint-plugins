'use strict';

const native = require('./native.js');

function implementedAngularEslintRuleNames() {
  return native.implementedAngularEslintRuleNames();
}

function scanAngularEslint(sourceText, filename = 'file.ts', options = {}) {
  if (typeof sourceText !== 'string') {
    throw new TypeError('sourceText must be a string.');
  }
  if (typeof filename !== 'string') {
    throw new TypeError('filename must be a string.');
  }

  return native.scanAngularEslint(sourceText, filename, normalizeOptions(options));
}

function normalizeOptions(options) {
  if (!options || typeof options !== 'object') {
    return {};
  }
  return {
    ruleNames: Array.isArray(options.ruleNames)
      ? options.ruleNames.filter((name) => typeof name === 'string' && name.length > 0)
      : undefined,
    options: options.options,
  };
}

module.exports = {
  implementedAngularEslintRuleNames,
  scanAngularEslint,
};
module.exports.default = module.exports;

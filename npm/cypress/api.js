'use strict';

const native = require('@oxlint-plugins/core').cypress;

function scanCypress(sourceText, filename = 'file.js', options = {}) {
  if (typeof sourceText !== 'string') {
    throw new TypeError('sourceText must be a string.');
  }
  if (typeof filename !== 'string') {
    throw new TypeError('filename must be a string.');
  }
  return native.scanCypress(sourceText, filename, {
    unsafeToChainMethods: normalizeStringArray(options.unsafeToChainMethods),
  });
}

function normalizeStringArray(values) {
  if (!Array.isArray(values)) {
    return [];
  }
  return values.filter((value) => typeof value === 'string' && value.length > 0);
}

module.exports = {
  implementedCypressRuleNames: native.implementedCypressRuleNames,
  scanCypress,
};
module.exports.default = module.exports;

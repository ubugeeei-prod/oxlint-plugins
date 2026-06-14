'use strict';

const native = require('./native.js');

function scanPostgresql(sourceText, options = {}) {
  if (typeof sourceText !== 'string') {
    throw new TypeError('sourceText must be a string.');
  }

  return native.scanPostgresql(sourceText, normalizeOptions(options));
}

function normalizeOptions(options) {
  if (!options || typeof options !== 'object') {
    return {};
  }

  return {
    ruleNames: normalizeStringList(options.ruleNames),
    options: options.options !== undefined ? options.options : undefined,
  };
}

function normalizeStringList(values) {
  if (!Array.isArray(values)) {
    return undefined;
  }
  return values.filter((value) => typeof value === 'string' && value.length > 0);
}

module.exports = {
  implementedPostgresqlRuleNames: native.implementedPostgresqlRuleNames,
  scanPostgresql,
};
module.exports.default = module.exports;

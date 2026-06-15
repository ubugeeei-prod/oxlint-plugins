'use strict';

const native = require('./native.js');

function scanPostgresql(sourceText, filename = 'schema.sql', options) {
  if (typeof sourceText !== 'string') {
    throw new TypeError('sourceText must be a string.');
  }
  if (typeof filename !== 'string') {
    throw new TypeError('filename must be a string.');
  }
  return native.scanPostgresql(sourceText, filename, normalizeOptions(options));
}

function normalizeOptions(options) {
  if (options == null) {
    return undefined;
  }
  if (typeof options !== 'object') {
    throw new TypeError('options must be an object when provided.');
  }
  return options;
}

module.exports = {
  implementedPostgresqlRuleNames: native.implementedPostgresqlRuleNames,
  scanPostgresql,
};
module.exports.default = module.exports;

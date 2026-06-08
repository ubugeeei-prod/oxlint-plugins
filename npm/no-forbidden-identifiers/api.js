'use strict';

const native = require('./native.js');

function scanForbiddenIdentifiers(sourceText, options) {
  if (typeof sourceText !== 'string') {
    throw new TypeError('sourceText must be a string.');
  }

  return native.scanForbiddenIdentifiers(sourceText, normalizeOptions(options));
}

function isForbiddenIdentifierName(name, options) {
  if (typeof name !== 'string') {
    throw new TypeError('name must be a string.');
  }

  return native.isForbiddenIdentifierName(name, normalizeOptions(options));
}

function normalizeOptions(options) {
  if (!options || !Array.isArray(options.names)) {
    return undefined;
  }

  return {
    names: options.names.filter((name) => typeof name === 'string' && name.length > 0),
  };
}

module.exports = {
  isForbiddenIdentifierName,
  scanForbiddenIdentifiers,
};
module.exports.default = module.exports;

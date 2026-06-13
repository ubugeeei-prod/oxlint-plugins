'use strict';

const native = require('./native.js');

function scanRegexp(sourceText, filename = 'file.js') {
  if (typeof sourceText !== 'string') {
    throw new TypeError('sourceText must be a string.');
  }
  if (typeof filename !== 'string') {
    throw new TypeError('filename must be a string.');
  }
  return native.scanRegexp(sourceText, filename);
}

module.exports = {
  implementedRegexpRuleNames: native.implementedRegexpRuleNames,
  scanRegexp,
};
module.exports.default = module.exports;

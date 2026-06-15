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

module.exports = {
  implementedPerfectionistRuleNames: native.implementedPerfectionistRuleNames,
  scanPerfectionist,
};
module.exports.default = module.exports;

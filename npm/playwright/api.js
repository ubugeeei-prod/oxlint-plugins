'use strict';

const native = require('./native.js');

function implementedPlaywrightRuleNames() {
  return native.implementedPlaywrightRuleNames();
}

function scanPlaywright(sourceText, filename = 'file.spec.ts') {
  if (typeof sourceText !== 'string') {
    throw new TypeError('sourceText must be a string.');
  }
  if (typeof filename !== 'string') {
    throw new TypeError('filename must be a string.');
  }

  return native.scanPlaywright(sourceText, filename);
}

module.exports = {
  implementedPlaywrightRuleNames,
  scanPlaywright,
};
module.exports.default = module.exports;

'use strict';

const native = require('./native.js');

function scanStorybook(sourceText, filename = 'file.stories.tsx', options = {}) {
  if (typeof sourceText !== 'string') {
    throw new TypeError('sourceText must be a string.');
  }
  if (typeof filename !== 'string') {
    throw new TypeError('filename must be a string.');
  }
  return native.scanStorybook(sourceText, filename, {
    ruleNames: normalizeStrings(options.ruleNames),
    installedAddons: normalizeStrings(options.installedAddons),
    ignoredAddons: normalizeStrings(options.ignoredAddons),
    packageJsonPath:
      typeof options.packageJsonPath === 'string' ? options.packageJsonPath : undefined,
  });
}

function normalizeStrings(values) {
  if (!Array.isArray(values)) {
    return undefined;
  }
  return values.filter((value) => typeof value === 'string' && value.length > 0);
}

module.exports = {
  implementedStorybookRuleNames: native.implementedStorybookRuleNames,
  scanStorybook,
};
module.exports.default = module.exports;

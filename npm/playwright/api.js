'use strict';

const native = require('./native.js');

function implementedPlaywrightRuleNames() {
  return native.implementedPlaywrightRuleNames();
}

function scanPlaywright(sourceText, filename = 'file.spec.ts', options = {}) {
  if (typeof sourceText !== 'string') {
    throw new TypeError('sourceText must be a string.');
  }
  if (typeof filename !== 'string') {
    throw new TypeError('filename must be a string.');
  }
  if (!options || typeof options !== 'object' || Array.isArray(options)) {
    throw new TypeError('options must be an object.');
  }

  return native.scanPlaywright(sourceText, filename, {
    expectAliases: stringList(options.expectAliases),
    restrictedLocators: listRestrictions(options.noRestrictedLocators, 'type'),
    restrictedMatchers: matcherRestrictions(options.noRestrictedMatchers),
    restrictedRoles: listRestrictions(options.noRestrictedRoles, 'role'),
  });
}

function listRestrictions(values, key) {
  if (!Array.isArray(values)) {
    return [];
  }
  return values.flatMap((value) => {
    if (typeof value === 'string') {
      return [{ value }];
    }
    if (!value || typeof value !== 'object' || typeof value[key] !== 'string') {
      return [];
    }
    return [
      {
        value: value[key],
        ...(typeof value.message === 'string' ? { message: value.message } : {}),
      },
    ];
  });
}

function matcherRestrictions(values) {
  if (!values || typeof values !== 'object' || Array.isArray(values)) {
    return [];
  }
  return Object.entries(values).flatMap(([value, message]) =>
    message === null || typeof message === 'string'
      ? [{ value, ...(message === null ? {} : { message }) }]
      : [],
  );
}

function stringList(values) {
  if (!Array.isArray(values)) {
    return [];
  }
  return values.filter((value) => typeof value === 'string');
}

module.exports = {
  implementedPlaywrightRuleNames,
  scanPlaywright,
};
module.exports.default = module.exports;

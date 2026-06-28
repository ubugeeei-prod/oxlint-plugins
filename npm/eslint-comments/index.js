'use strict';

const { eslintCompatPlugin } = require('@oxlint/plugins');

const noUnlimitedDisable = require('./src/no-unlimited-disable.js');

const PLUGIN_NAME = 'eslint-comments';

const plugin = eslintCompatPlugin({
  meta: {
    name: PLUGIN_NAME,
    version: '0.0.0',
  },
  rules: {
    'no-unlimited-disable': noUnlimitedDisable,
  },
});

module.exports = plugin;
module.exports.default = plugin;

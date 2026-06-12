'use strict';

// Convenience bundle for the whole @oxlint-plugins suite.
//
// Installing this one package pulls in the shared native core and every plugin
// facade, and exposes a single combined oxlint plugin so the suite can be
// registered with one `jsPlugins` entry:
//
//   // oxlint.config.jsonc
//   { "jsPlugins": [{ "name": "oxlint", "specifier": "@oxlint-plugins/oxlint" }] }
//
// Rules keep their upstream names under the `oxlint/` namespace, e.g.
// `oxlint/no-unlimited-disable`. If you prefer each plugin's own namespace,
// depend on the individual `@oxlint-plugins/oxlint-plugin-*` packages instead;
// they are re-exported here as `plugins`.

const { eslintCompatPlugin } = require('@oxlint/plugins');
const eslintComments = require('@oxlint-plugins/oxlint-plugin-eslint-comments');
const noForbiddenIdentifiers = require('@oxlint-plugins/oxlint-plugin-no-forbidden-identifiers');
const stylistic = require('@oxlint-plugins/oxlint-plugin-stylistic');

const PLUGIN_NAME = 'oxlint';

// Each ported plugin, keyed by its own oxlint plugin name.
const plugins = {
  'eslint-comments': eslintComments,
  'no-forbidden-identifiers': noForbiddenIdentifiers,
  stylistic,
};

// Union of every plugin's rules, exposed under the single `oxlint` namespace.
// Rule names are unique across plugins today; if two ever collide this throws so
// the conflict is caught at load time rather than silently dropping a rule.
const rules = {};
for (const [pluginName, plugin] of Object.entries(plugins)) {
  for (const [ruleName, rule] of Object.entries(plugin.rules ?? {})) {
    if (Object.hasOwn(rules, ruleName)) {
      throw new Error(
        `@oxlint-plugins/oxlint: rule name "${ruleName}" from "${pluginName}" collides with another plugin. ` +
          'Register the conflicting plugins individually instead of via the bundle.',
      );
    }
    rules[ruleName] = rule;
  }
}

// Re-key each plugin's own `recommended` rules from `<plugin>/<rule>` to
// `oxlint/<rule>`, so the bundle's recommended config matches its namespace.
const recommendedRules = {};
for (const plugin of Object.values(plugins)) {
  const source = plugin.configs?.recommended?.rules ?? {};
  for (const [ruleId, severity] of Object.entries(source)) {
    const ruleName = ruleId.slice(ruleId.indexOf('/') + 1);
    recommendedRules[`${PLUGIN_NAME}/${ruleName}`] = severity;
  }
}

const plugin = eslintCompatPlugin({
  meta: {
    name: PLUGIN_NAME,
    version: '0.0.0',
  },
  rules,
  configs: {
    recommended: {
      plugins: [PLUGIN_NAME],
      rules: recommendedRules,
    },
  },
});

// Expose the individual plugins for users who want per-plugin namespaces.
plugin.plugins = plugins;

module.exports = plugin;
module.exports.default = plugin;

'use strict';

const { eslintCompatPlugin } = require('@oxlint/plugins');
const native = require('./native.js');

const PLUGIN_NAME = 'no-forbidden-identifiers';
const RULE_NAME = 'no-forbidden-identifiers';

function normalizeOptions(rawOptions) {
  if (!rawOptions || !Array.isArray(rawOptions.names)) {
    return undefined;
  }

  const names = [];
  for (const name of rawOptions.names) {
    if (typeof name === 'string' && name.length > 0) {
      names.push(name);
    }
  }

  return names.length > 0 ? { names } : undefined;
}

const noForbiddenIdentifiers = {
  meta: {
    type: 'problem',
    docs: {
      description: 'disallow project-specific identifier names',
      recommended: false,
      url: 'https://github.com/ubugeeei-prod/oxlint-plugins/tree/main/npm/no-forbidden-identifiers',
    },
    schema: [
      {
        type: 'object',
        additionalProperties: false,
        properties: {
          names: {
            type: 'array',
            minItems: 1,
            items: { type: 'string', minLength: 1 },
          },
        },
      },
    ],
    messages: {
      forbiddenIdentifier: "Identifier '{{name}}' is reserved by this project policy.",
    },
  },
  createOnce(context) {
    const options = normalizeOptions(context.options?.[0]);
    let activeNames;

    return {
      before() {
        const sourceText = context.sourceCode?.text;
        if (typeof sourceText !== 'string' || sourceText.length === 0) {
          return false;
        }

        const matches = native.scanForbiddenIdentifiers(sourceText, options);
        if (matches.length === 0) {
          return false;
        }

        activeNames = new Set(matches);
      },
      Identifier(node) {
        if (!activeNames || typeof node.name !== 'string' || !activeNames.has(node.name)) {
          return;
        }

        context.report({
          node,
          messageId: 'forbiddenIdentifier',
          data: { name: node.name },
        });
      },
      after() {
        activeNames = undefined;
      },
    };
  },
};

const plugin = eslintCompatPlugin({
  meta: {
    name: PLUGIN_NAME,
    version: '0.0.0',
  },
  rules: {
    [RULE_NAME]: noForbiddenIdentifiers,
  },
  configs: {
    recommended: {
      plugins: [PLUGIN_NAME],
      rules: {
        [`${PLUGIN_NAME}/${RULE_NAME}`]: 'error',
      },
    },
  },
});

module.exports = plugin;
module.exports.default = plugin;

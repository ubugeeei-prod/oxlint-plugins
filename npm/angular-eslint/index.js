'use strict';

// Oxlint plugin port of @angular-eslint/eslint-plugin (MIT).
// The JavaScript layer is an Oxlint/NAPI adapter; Angular-focused scans run in
// Rust through Oxc-backed source parsing and fast structural pattern checks.

const { eslintCompatPlugin } = require('@oxlint/plugins');
const { implementedAngularEslintRuleNames, scanAngularEslint } = require('./api.js');

const PLUGIN_NAME = '@angular-eslint';
const DOCS_BASE =
  'https://github.com/angular-eslint/angular-eslint/blob/main/packages/eslint-plugin/docs/rules';
const diagnosticsCache = new WeakMap();
const implementedRuleNames = Object.freeze(implementedAngularEslintRuleNames());
const selectorRuleNames = new Set(['component-selector', 'directive-selector']);
const classSuffixRuleNames = new Set(['component-class-suffix', 'directive-class-suffix']);
const prefixRuleNames = new Set(['no-input-prefix', 'pipe-prefix']);
const inlineDeclarationRuleName = 'component-max-inline-declarations';
const consistentComponentStylesRuleName = 'consistent-component-styles';
const inputRenameRuleName = 'no-input-rename';
const optionAwareRuleNames = new Set([
  ...selectorRuleNames,
  ...classSuffixRuleNames,
  ...prefixRuleNames,
  inlineDeclarationRuleName,
  consistentComponentStylesRuleName,
  inputRenameRuleName,
]);

const consistentComponentStylesSchema = [
  {
    type: 'string',
    enum: ['array', 'string'],
  },
];

const consistentComponentStylesMessages = {
  useStyleUrl: 'Use `styleUrl` instead of `styleUrls` for a single stylesheet',
  useStyleUrls: 'Use `styleUrls` instead of `styleUrl`',
  useStylesArray: 'Use a `string[]` instead of a `string` for the `styles` property',
  useStylesString: 'Use a `string` instead of a `string[]` for the `styles` property',
};

const inlineDeclarationSchema = [
  {
    type: 'object',
    properties: {
      template: { minimum: 0, type: 'number' },
      styles: { minimum: 0, type: 'number' },
      animations: { minimum: 0, type: 'number' },
    },
    additionalProperties: false,
  },
];

const inlineDeclarationMessages = {
  componentMaxInlineDeclarations:
    '`{{propertyType}}` has too many lines ({{lineCount}}). Maximum allowed is {{max}}',
};

const classSuffixSchema = [
  {
    type: 'object',
    properties: {
      suffixes: {
        type: 'array',
        items: { type: 'string' },
      },
    },
    additionalProperties: false,
  },
];

const classSuffixMessages = {
  'component-class-suffix': {
    componentClassSuffix:
      'Component class names should end with one of these suffixes: {{suffixes}}',
  },
  'directive-class-suffix': {
    directiveClassSuffix:
      'Directive class names should end with one of these suffixes: {{suffixes}}',
  },
};

const prefixSchemas = {
  'no-input-prefix': [
    {
      type: 'object',
      properties: {
        prefixes: {
          type: 'array',
          items: { type: 'string' },
        },
      },
      additionalProperties: false,
    },
  ],
  'pipe-prefix': [
    {
      type: 'object',
      properties: {
        prefixes: {
          type: 'array',
          items: { type: 'string' },
          uniqueItems: true,
        },
      },
      additionalProperties: false,
    },
  ],
};

const prefixMessages = {
  'no-input-prefix': {
    noInputPrefix:
      'Input bindings, including aliases, should not be named, nor prefixed by {{prefixes}}',
  },
  'pipe-prefix': {
    pipePrefix: '@Pipes should be prefixed with {{prefixes}}',
    selectorAfterPrefixFailure: '@Pipes should have a selector after the {{prefixes}} prefix',
  },
};

const inputRenameSchema = [
  {
    type: 'object',
    properties: {
      allowedNames: {
        type: 'array',
        items: { type: 'string' },
        description: 'A list with allowed input names',
        uniqueItems: true,
      },
    },
    additionalProperties: false,
  },
];

const inputRenameMessages = {
  noInputRename:
    'Input bindings should not be aliased (https://angular.dev/guide/components/inputs#choosing-input-names)',
  suggestRemoveAliasName: 'Remove alias name',
  suggestReplaceOriginalNameWithAliasName: 'Remove alias name and use it as the original name',
};

const selectorConfigSchema = {
  type: 'object',
  properties: {
    type: {
      oneOf: [
        { type: 'string', enum: ['element', 'attribute'] },
        {
          type: 'array',
          items: { type: 'string', enum: ['element', 'attribute'] },
          minItems: 1,
          uniqueItems: true,
        },
      ],
    },
    prefix: {
      oneOf: [{ type: 'string' }, { type: 'array', items: { type: 'string' } }],
    },
    style: { type: 'string', enum: ['camelCase', 'kebab-case'] },
  },
  required: ['type', 'style'],
  additionalProperties: false,
};

const selectorSchema = [
  {
    oneOf: [
      selectorConfigSchema,
      {
        type: 'array',
        items: {
          ...selectorConfigSchema,
          properties: {
            ...selectorConfigSchema.properties,
            type: { type: 'string', enum: ['element', 'attribute'] },
          },
        },
        minItems: 1,
        maxItems: 2,
      },
    ],
  },
];

const selectorMessages = {
  'component-selector': {
    prefixFailure:
      'The selector should start with one of these prefixes: {{prefix}} (https://angular.dev/style-guide#choosing-component-selectors)',
    styleFailure:
      'The selector should be {{style}} (https://angular.dev/style-guide#choosing-component-selectors)',
    styleAndPrefixFailure:
      'The selector should be {{style}} and start with one of these prefixes: {{prefix}} (https://angular.dev/style-guide#choosing-component-selectors and https://angular.dev/style-guide#choosing-component-selectors)',
    typeFailure:
      'The selector should be used as an {{type}} (https://angular.dev/style-guide#choosing-component-selectors)',
    shadowDomEncapsulatedStyleFailure:
      'The selector of a ShadowDom-encapsulated component should be `kebab-case` (https://github.com/angular-eslint/angular-eslint/issues/534)',
    selectorAfterPrefixFailure: 'There should be a selector after the {{prefix}} prefix',
  },
  'directive-selector': {
    prefixFailure: 'The selector should start with one of these prefixes: {{prefix}}',
    styleFailure: 'The selector should be {{style}}',
    typeFailure: 'The selector should be used as an {{type}}',
    selectorAfterPrefixFailure: 'There should be a selector after the {{prefix}} prefix',
  },
};

const problemRules = new Set([
  'computed-must-return',
  'contextual-lifecycle',
  'no-async-lifecycle-method',
  'no-attribute-decorator',
  'no-developer-preview',
  'no-empty-lifecycle-method',
  'no-experimental',
  'no-lifecycle-call',
  'require-lifecycle-on-prototype',
]);

const rules = Object.freeze(
  Object.fromEntries(
    implementedRuleNames.map((ruleName) => [ruleName, createAngularEslintRule(ruleName)]),
  ),
);

const allRuleConfig = Object.freeze(
  Object.fromEntries(
    implementedRuleNames.map((ruleName) => [`${PLUGIN_NAME}/${ruleName}`, 'error']),
  ),
);

const plugin = eslintCompatPlugin({
  meta: {
    name: PLUGIN_NAME,
    version: '0.0.0',
  },
  rules,
  rulesConfig: Object.fromEntries(implementedRuleNames.map((ruleName) => [ruleName, 0])),
  configs: {
    all: {
      name: `${PLUGIN_NAME}/all`,
      plugins: [PLUGIN_NAME],
      rules: allRuleConfig,
    },
  },
});

plugin.implementedAngularEslintRuleNames = implementedRuleNames;
plugin.scanAngularEslint = scanAngularEslint;

function createAngularEslintRule(ruleName) {
  const isSelectorRule = selectorRuleNames.has(ruleName);
  const isClassSuffixRule = classSuffixRuleNames.has(ruleName);
  const isPrefixRule = prefixRuleNames.has(ruleName);
  const isInlineDeclarationRule = ruleName === inlineDeclarationRuleName;
  const isConsistentComponentStylesRule = ruleName === consistentComponentStylesRuleName;
  const isInputRenameRule = ruleName === inputRenameRuleName;
  return {
    meta: {
      type: problemRules.has(ruleName) ? 'problem' : 'suggestion',
      docs: {
        description: isConsistentComponentStylesRule
          ? 'Ensures consistent usage of `styles`/`styleUrls`/`styleUrl` within Component metadata'
          : `enforce angular eslint ${ruleName.replaceAll('-', ' ')}`,
        category: 'Best Practices',
        recommended: false,
        url: `${DOCS_BASE}/${ruleName}.md`,
      },
      messages: isSelectorRule
        ? selectorMessages[ruleName]
        : isClassSuffixRule
          ? classSuffixMessages[ruleName]
          : isPrefixRule
            ? prefixMessages[ruleName]
            : isInlineDeclarationRule
              ? inlineDeclarationMessages
              : isConsistentComponentStylesRule
                ? consistentComponentStylesMessages
                : isInputRenameRule
                  ? inputRenameMessages
                  : {
                      unexpected: 'Unexpected Angular pattern.',
                    },
      schema: isSelectorRule
        ? selectorSchema
        : isClassSuffixRule
          ? classSuffixSchema
          : isPrefixRule
            ? prefixSchemas[ruleName]
            : isInlineDeclarationRule
              ? inlineDeclarationSchema
              : isConsistentComponentStylesRule
                ? consistentComponentStylesSchema
                : isInputRenameRule
                  ? inputRenameSchema
                  : [],
      ...(isConsistentComponentStylesRule ? { fixable: 'code' } : {}),
      ...(isInputRenameRule ? { fixable: 'code', hasSuggestions: true } : {}),
    },
    createOnce(context) {
      return {
        Program() {
          for (const diagnostic of diagnosticsForRule(context, ruleName)) {
            reportDiagnostic(context, diagnostic);
          }
        },
      };
    },
  };
}

function diagnosticsForRule(context, ruleName) {
  if (optionAwareRuleNames.has(ruleName)) {
    const sourceText = sourceTextForContext(context);
    const filename = typeof context.filename === 'string' ? context.filename : 'file.ts';
    return scanAngularEslint(sourceText, filename, {
      ruleNames: [ruleName],
      options: context.options || [],
    });
  }
  return diagnosticsForContext(context).filter((diagnostic) => diagnostic.ruleName === ruleName);
}

function diagnosticsForContext(context) {
  const sourceCode = context.sourceCode || {};
  const sourceText = sourceTextForContext(context);
  const filename = typeof context.filename === 'string' ? context.filename : 'file.ts';
  let cached = diagnosticsCache.get(sourceCode);

  if (cached && cached.sourceText === sourceText && cached.filename === filename) {
    return cached.diagnostics;
  }

  const diagnostics = scanAngularEslint(sourceText, filename);
  cached = { sourceText, filename, diagnostics };
  diagnosticsCache.set(sourceCode, cached);
  return diagnostics;
}

function reportDiagnostic(context, diagnostic) {
  context.report({
    messageId: diagnostic.messageId,
    data: Object.fromEntries(diagnostic.data.map(({ key, value }) => [key, value])),
    loc: {
      start: {
        line: diagnostic.loc.startLine,
        column: diagnostic.loc.startColumn,
      },
      end: {
        line: diagnostic.loc.endLine,
        column: diagnostic.loc.endColumn,
      },
    },
  });
}

function sourceTextForContext(context) {
  const sourceCode = context.sourceCode || {};
  if (typeof sourceCode.getText === 'function') {
    return sourceCode.getText();
  }
  if (typeof sourceCode.text === 'string') {
    return sourceCode.text;
  }
  return '';
}

module.exports = plugin;
module.exports.default = plugin;
module.exports.implementedAngularEslintRuleNames = implementedRuleNames;
module.exports.scanAngularEslint = scanAngularEslint;

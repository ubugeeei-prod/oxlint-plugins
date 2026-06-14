'use strict';

// Oxlint plugin port of @eslint/markdown (MIT).
// Markdown source scanning, rule option handling, and autofix range calculation
// run in Rust through NAPI-RS. Oxlint 1.68 does not pass .md files to jsPlugins,
// so this package exposes the adapter and native API while keeping CLI
// integration disabled in status metadata.

const { eslintCompatPlugin } = require('@oxlint/plugins');
const { implementedEslintMarkdownRuleNames, scanEslintMarkdown } = require('./api.js');

const PLUGIN_NAME = 'markdown';
const DOCS_BASE = 'https://github.com/ubugeeei-prod/oxlint-plugins/tree/main/npm/eslint-markdown';
const diagnosticsCache = new WeakMap();

const recommendedRuleNames = Object.freeze([
  'fenced-code-language',
  'heading-increment',
  'no-duplicate-definitions',
  'no-empty-definitions',
  'no-empty-images',
  'no-empty-links',
  'no-invalid-label-refs',
  'no-missing-atx-heading-space',
  'no-missing-label-refs',
  'no-missing-link-fragments',
  'no-multiple-h1',
  'no-reference-like-urls',
  'no-reversed-media-syntax',
  'no-space-in-emphasis',
  'no-unused-definitions',
  'require-alt-text',
  'table-column-count',
]);

const ruleDescriptions = Object.freeze({
  'fenced-code-language': 'Require languages for fenced code blocks',
  'fenced-code-meta': 'Require or disallow metadata for fenced code blocks',
  'heading-increment': 'Enforce heading levels increment by one',
  'no-bare-urls': 'Disallow bare URLs',
  'no-duplicate-definitions': 'Disallow duplicate definitions',
  'no-duplicate-headings': 'Disallow duplicate headings in the same document',
  'no-empty-definitions': 'Disallow empty definitions',
  'no-empty-images': 'Disallow empty images',
  'no-empty-links': 'Disallow empty links',
  'no-html': 'Disallow HTML tags',
  'no-invalid-label-refs': 'Disallow invalid label references',
  'no-missing-atx-heading-space': 'Require spaces around ATX heading hash characters',
  'no-missing-label-refs': 'Disallow missing label references',
  'no-missing-link-fragments': "Disallow link fragments that don't exist in the document",
  'no-multiple-h1': 'Disallow multiple H1 headings in the same document',
  'no-reference-like-urls': 'Disallow URLs that match defined reference identifiers',
  'no-reversed-media-syntax': 'Disallow reversed link and image syntax',
  'no-space-in-emphasis': 'Disallow spaces around emphasis markers',
  'no-unused-definitions': 'Disallow unused definitions',
  'require-alt-text': 'Require alternative text for images',
  'table-column-count':
    'Disallow data rows in a GitHub Flavored Markdown table from having more cells than the header row',
});

const ruleMessages = Object.freeze({
  'fenced-code-language': {
    missingLanguage: 'Missing code block language.',
    disallowedLanguage: 'Code block language "{{lang}}" is not allowed.',
  },
  'fenced-code-meta': {
    missingMetadata: 'Missing code block metadata.',
    disallowedMetadata: 'Code block metadata is not allowed.',
  },
  'heading-increment': {
    skippedHeading: 'Heading level skipped from {{fromLevel}} to {{toLevel}}.',
  },
  'no-bare-urls': {
    bareUrl: 'Unexpected bare URL. Use autolink (<URL>) or link ([text](URL)) instead.',
  },
  'no-duplicate-definitions': {
    duplicateDefinition:
      'Unexpected duplicate definition `{{ identifier }}` (label: `{{ label }}`) found. First defined at line {{ firstLine }} (label: `{{ firstLabel }}`).',
    duplicateFootnoteDefinition:
      'Unexpected duplicate footnote definition `{{ identifier }}` (label: `{{ label }}`) found. First defined at line {{ firstLine }} (label: `{{ firstLabel }}`).',
  },
  'no-duplicate-headings': {
    duplicateHeading: 'Duplicate heading "{{text}}" found.',
  },
  'no-empty-definitions': {
    emptyDefinition: 'Unexpected empty definition `{{ identifier }}` (label: `{{ label }}`) found.',
    emptyFootnoteDefinition:
      'Unexpected empty footnote definition `{{ identifier }}` (label: `{{ label }}`) found.',
  },
  'no-empty-images': {
    emptyImage: 'Unexpected empty image found.',
  },
  'no-empty-links': {
    emptyLink: 'Unexpected empty link found.',
  },
  'no-html': {
    disallowedElement: 'HTML element "{{name}}" is not allowed.',
  },
  'no-invalid-label-refs': {
    invalidLabelRef: "Label reference '{{label}}' is invalid due to white space between [ and ].",
  },
  'no-missing-atx-heading-space': {
    missingSpace: 'Missing space {{position}} hash(es) on ATX style heading.',
  },
  'no-missing-label-refs': {
    notFound: "Label reference '{{label}}' not found.",
  },
  'no-missing-link-fragments': {
    invalidFragment:
      "Link fragment '#{{fragment}}' does not reference a heading or anchor in this document.",
  },
  'no-multiple-h1': {
    multipleH1: 'Unexpected additional H1 heading found.',
  },
  'no-reference-like-urls': {
    referenceLikeUrl:
      "Unexpected resource {{type}} ('{{prefix}}[text](url)') with URL that matches a definition identifier. Use '[text][id]' syntax instead.",
  },
  'no-reversed-media-syntax': {
    reversedSyntax: 'Unexpected reversed syntax found. Use [label](URL) syntax instead.',
  },
  'no-space-in-emphasis': {
    spaceInEmphasis: 'Unexpected space around emphasis marker.',
  },
  'no-unused-definitions': {
    unusedDefinition:
      'Unexpected unused definition `{{ identifier }}` (label: `{{ label }}`) found.',
    unusedFootnoteDefinition:
      'Unexpected unused footnote definition `{{ identifier }}` (label: `{{ label }}`) found.',
  },
  'require-alt-text': {
    altTextRequired: 'Alternative text for image is required.',
  },
  'table-column-count': {
    extraCells:
      'Table column count mismatch (Expected: {{expectedCells}}, Actual: {{actualCells}}), extra data starting here will be ignored.',
    missingCells:
      'Table column count mismatch (Expected: {{expectedCells}}, Actual: {{actualCells}}), row might be missing data.',
  },
});

const fixableRules = Object.freeze({
  'no-bare-urls': 'code',
  'no-missing-atx-heading-space': 'whitespace',
  'no-reference-like-urls': 'code',
  'no-reversed-media-syntax': 'code',
  'no-space-in-emphasis': 'whitespace',
});

const stringArraySchema = Object.freeze({
  type: 'array',
  items: { type: 'string' },
  uniqueItems: true,
});
const definitionOptionsSchema = Object.freeze([
  {
    type: 'object',
    properties: {
      allowDefinitions: stringArraySchema,
      allowFootnoteDefinitions: stringArraySchema,
      checkFootnoteDefinitions: { type: 'boolean' },
    },
    additionalProperties: false,
  },
]);

const ruleSchemas = Object.freeze({
  'fenced-code-language': [
    {
      type: 'object',
      properties: {
        required: stringArraySchema,
      },
      additionalProperties: false,
    },
  ],
  'fenced-code-meta': [{ enum: ['always', 'never'] }],
  'heading-increment': [
    {
      type: 'object',
      properties: {
        frontmatterTitle: { type: 'string' },
      },
      additionalProperties: false,
    },
  ],
  'no-duplicate-definitions': definitionOptionsSchema,
  'no-duplicate-headings': [
    {
      type: 'object',
      properties: {
        checkSiblingsOnly: { type: 'boolean' },
      },
      additionalProperties: false,
    },
  ],
  'no-empty-definitions': definitionOptionsSchema,
  'no-html': [
    {
      type: 'object',
      properties: {
        allowed: stringArraySchema,
        allowedIgnoreCase: { type: 'boolean' },
      },
      additionalProperties: false,
    },
  ],
  'no-missing-atx-heading-space': [
    {
      type: 'object',
      properties: {
        checkClosedHeadings: { type: 'boolean' },
      },
      additionalProperties: false,
    },
  ],
  'no-missing-label-refs': [
    {
      type: 'object',
      properties: {
        allowLabels: stringArraySchema,
      },
      additionalProperties: false,
    },
  ],
  'no-missing-link-fragments': [
    {
      type: 'object',
      properties: {
        ignoreCase: { type: 'boolean' },
        allowPattern: { type: 'string' },
      },
      additionalProperties: false,
    },
  ],
  'no-multiple-h1': [
    {
      type: 'object',
      properties: {
        frontmatterTitle: { type: 'string' },
      },
      additionalProperties: false,
    },
  ],
  'no-space-in-emphasis': [
    {
      type: 'object',
      properties: {
        checkStrikethrough: { type: 'boolean' },
      },
      additionalProperties: false,
    },
  ],
  'no-unused-definitions': definitionOptionsSchema,
  'table-column-count': [
    {
      type: 'object',
      properties: {
        checkMissingCells: { type: 'boolean' },
      },
      additionalProperties: false,
    },
  ],
});

const implementedRuleNames = Object.freeze(implementedEslintMarkdownRuleNames());
const recommendedRuleConfig = Object.freeze(
  Object.fromEntries(
    recommendedRuleNames.map((ruleName) => [`${PLUGIN_NAME}/${ruleName}`, 'error']),
  ),
);
const allRuleConfig = Object.freeze(
  Object.fromEntries(
    implementedRuleNames.map((ruleName) => [`${PLUGIN_NAME}/${ruleName}`, 'error']),
  ),
);

const rules = Object.freeze(
  Object.fromEntries(
    implementedRuleNames.map((ruleName) => [ruleName, createMarkdownRule(ruleName)]),
  ),
);

const plugin = eslintCompatPlugin({
  meta: {
    name: '@eslint/markdown',
    version: '8.0.2',
  },
  rules,
  rulesConfig: Object.fromEntries(implementedRuleNames.map((ruleName) => [ruleName, 0])),
  configs: {
    recommended: configFromRuleConfig('recommended', recommendedRuleConfig),
    all: configFromRuleConfig('all', allRuleConfig),
  },
});

plugin.implementedEslintMarkdownRuleNames = implementedRuleNames;
plugin.scanEslintMarkdown = scanEslintMarkdown;

function configFromRuleConfig(name, ruleConfig) {
  return {
    name: `${PLUGIN_NAME}/${name}`,
    plugins: [PLUGIN_NAME],
    rules: { ...ruleConfig },
  };
}

function createMarkdownRule(ruleName) {
  return {
    meta: {
      type: 'problem',
      docs: {
        description: ruleDescriptions[ruleName],
        recommended: recommendedRuleNames.includes(ruleName),
        url: `${DOCS_BASE}#${ruleName}`,
      },
      fixable: fixableRules[ruleName],
      messages: ruleMessages[ruleName],
      schema: ruleSchemas[ruleName] ?? [],
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
  const sourceCode = context.sourceCode ?? context.getSourceCode();
  let bySource = diagnosticsCache.get(sourceCode);
  if (!bySource) {
    bySource = new Map();
    diagnosticsCache.set(sourceCode, bySource);
  }

  const scanOptions = optionsForRule(ruleName, context.options?.[0]);
  // `languageOptions.math` enables `$...$` math parsing (off by default upstream).
  if (context.languageOptions?.math === true) {
    scanOptions.math = true;
  }
  const cacheKey = `${ruleName}\0${JSON.stringify(scanOptions)}`;
  let diagnostics = bySource.get(cacheKey);
  if (!diagnostics) {
    diagnostics = scanEslintMarkdown(sourceCode.text, scanOptions).filter(
      (diagnostic) => diagnostic.ruleName === ruleName,
    );
    bySource.set(cacheKey, diagnostics);
  }

  return diagnostics;
}

function optionsForRule(ruleName, rawOptions) {
  const ruleOptions = rawOptions && typeof rawOptions === 'object' ? rawOptions : {};
  const scanOptions = { ruleNames: [ruleName] };

  switch (ruleName) {
    case 'fenced-code-language':
      scanOptions.requiredCodeLanguages = stringList(ruleOptions.required);
      break;
    case 'fenced-code-meta':
      scanOptions.fencedCodeMetaMode = rawOptions === 'never' ? 'never' : 'always';
      break;
    case 'heading-increment':
    case 'no-multiple-h1':
      if (typeof ruleOptions.frontmatterTitle === 'string') {
        scanOptions.frontmatterTitle = ruleOptions.frontmatterTitle;
      }
      break;
    case 'no-duplicate-definitions':
    case 'no-empty-definitions':
    case 'no-unused-definitions':
      scanOptions.allowDefinitions = stringList(ruleOptions.allowDefinitions);
      scanOptions.allowFootnoteDefinitions = stringList(ruleOptions.allowFootnoteDefinitions);
      scanOptions.checkFootnoteDefinitions = booleanOption(ruleOptions.checkFootnoteDefinitions);
      break;
    case 'no-duplicate-headings':
      scanOptions.checkDuplicateHeadingsSiblingsOnly = booleanOption(ruleOptions.checkSiblingsOnly);
      break;
    case 'no-html':
      scanOptions.allowedHtml = stringList(ruleOptions.allowed);
      scanOptions.allowedHtmlIgnoreCase = booleanOption(ruleOptions.allowedIgnoreCase);
      break;
    case 'no-missing-atx-heading-space':
      scanOptions.checkClosedHeadings = booleanOption(ruleOptions.checkClosedHeadings);
      break;
    case 'no-missing-label-refs':
      scanOptions.allowLabels = stringList(ruleOptions.allowLabels);
      break;
    case 'no-missing-link-fragments':
      scanOptions.ignoreFragmentCase = booleanOption(ruleOptions.ignoreCase);
      if (typeof ruleOptions.allowPattern === 'string') {
        scanOptions.allowFragmentPattern = ruleOptions.allowPattern;
      }
      break;
    case 'no-space-in-emphasis':
      scanOptions.checkStrikethrough = booleanOption(ruleOptions.checkStrikethrough);
      break;
    case 'table-column-count':
      scanOptions.checkMissingTableCells = booleanOption(ruleOptions.checkMissingCells);
      break;
  }

  return scanOptions;
}

function stringList(values) {
  return Array.isArray(values)
    ? values.filter((value) => typeof value === 'string' && value.length > 0)
    : undefined;
}

function booleanOption(value) {
  return typeof value === 'boolean' ? value : undefined;
}

function dataForReport(data) {
  const out = {};
  for (const [key, value] of Object.entries(data ?? {})) {
    if (value != null) {
      out[key] = String(value);
    }
  }
  if (out.linkType != null) {
    out.type = out.linkType;
  }
  return out;
}

function reportDiagnostic(context, diagnostic) {
  const report = {
    messageId: diagnostic.messageId,
    data: dataForReport(diagnostic.data),
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
  };

  if (diagnostic.fix) {
    report.fix = (fixer) =>
      fixer.replaceTextRange(
        [diagnostic.fix.start, diagnostic.fix.end],
        diagnostic.fix.replacement,
      );
  }

  context.report(report);
}

module.exports = plugin;
module.exports.default = plugin;

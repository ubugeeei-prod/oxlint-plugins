'use strict';

const native = require('./native.js');

function scanEslintMarkdown(sourceText, options = {}) {
  if (typeof sourceText !== 'string') {
    throw new TypeError('sourceText must be a string.');
  }

  return native.scanEslintMarkdown(sourceText, normalizeOptions(options));
}

function normalizeOptions(options) {
  if (!options || typeof options !== 'object') {
    return {};
  }

  return {
    ruleNames: normalizeStringList(options.ruleNames),
    requiredCodeLanguages: normalizeStringList(options.requiredCodeLanguages),
    fencedCodeMetaMode:
      options.fencedCodeMetaMode === 'never' || options.fencedCodeMetaMode === 'always'
        ? options.fencedCodeMetaMode
        : undefined,
    frontmatterTitle:
      typeof options.frontmatterTitle === 'string' ? options.frontmatterTitle : undefined,
    checkClosedHeadings: normalizeBoolean(options.checkClosedHeadings),
    checkStrikethrough: normalizeBoolean(options.checkStrikethrough),
    allowedHtml: normalizeStringList(options.allowedHtml),
    allowedHtmlIgnoreCase: normalizeBoolean(options.allowedHtmlIgnoreCase),
    allowLabels: normalizeStringList(options.allowLabels),
    allowDefinitions: normalizeStringList(options.allowDefinitions),
    allowFootnoteDefinitions: normalizeStringList(options.allowFootnoteDefinitions),
    checkFootnoteDefinitions: normalizeBoolean(options.checkFootnoteDefinitions),
    checkDuplicateHeadingsSiblingsOnly: normalizeBoolean(
      options.checkDuplicateHeadingsSiblingsOnly,
    ),
    ignoreFragmentCase: normalizeBoolean(options.ignoreFragmentCase),
    allowFragmentPattern:
      typeof options.allowFragmentPattern === 'string' ? options.allowFragmentPattern : undefined,
    checkMissingTableCells: normalizeBoolean(options.checkMissingTableCells),
    math: normalizeBoolean(options.math),
  };
}

function normalizeStringList(values) {
  if (!Array.isArray(values)) {
    return undefined;
  }
  return values.filter((value) => typeof value === 'string' && value.length > 0);
}

function normalizeBoolean(value) {
  return typeof value === 'boolean' ? value : undefined;
}

module.exports = {
  implementedEslintMarkdownRuleNames: native.implementedEslintMarkdownRuleNames,
  scanEslintMarkdown,
};
module.exports.default = module.exports;

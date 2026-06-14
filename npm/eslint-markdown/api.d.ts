export type EslintMarkdownRuleName =
  | 'fenced-code-language'
  | 'fenced-code-meta'
  | 'heading-increment'
  | 'no-bare-urls'
  | 'no-duplicate-definitions'
  | 'no-duplicate-headings'
  | 'no-empty-definitions'
  | 'no-empty-images'
  | 'no-empty-links'
  | 'no-html'
  | 'no-invalid-label-refs'
  | 'no-missing-atx-heading-space'
  | 'no-missing-label-refs'
  | 'no-missing-link-fragments'
  | 'no-multiple-h1'
  | 'no-reference-like-urls'
  | 'no-reversed-media-syntax'
  | 'no-space-in-emphasis'
  | 'no-unused-definitions'
  | 'require-alt-text'
  | 'table-column-count';

export type DiagnosticData = {
  lang?: string;
  name?: string;
  identifier?: string;
  label?: string;
  firstLine?: number;
  firstLabel?: string;
  fromLevel?: number;
  toLevel?: number;
  position?: string;
  text?: string;
  linkType?: string;
  prefix?: string;
  fragment?: string;
  expectedCells?: number;
  actualCells?: number;
};

export type DiagnosticLoc = {
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
};

export type DiagnosticFix = {
  start: number;
  end: number;
  replacement: string;
};

export type Diagnostic = {
  ruleName: EslintMarkdownRuleName;
  messageId: string;
  data: DiagnosticData;
  loc: DiagnosticLoc;
  fix?: DiagnosticFix;
};

export type ScanEslintMarkdownOptions = {
  ruleNames?: readonly string[];
  requiredCodeLanguages?: readonly string[];
  fencedCodeMetaMode?: 'always' | 'never';
  frontmatterTitle?: string;
  checkClosedHeadings?: boolean;
  checkStrikethrough?: boolean;
  allowedHtml?: readonly string[];
  allowedHtmlIgnoreCase?: boolean;
  allowLabels?: readonly string[];
  allowDefinitions?: readonly string[];
  allowFootnoteDefinitions?: readonly string[];
  checkFootnoteDefinitions?: boolean;
  checkDuplicateHeadingsSiblingsOnly?: boolean;
  ignoreFragmentCase?: boolean;
  allowFragmentPattern?: string;
  checkMissingTableCells?: boolean;
  math?: boolean;
  frontmatter?: boolean;
};

export function implementedEslintMarkdownRuleNames(): EslintMarkdownRuleName[];

export function scanEslintMarkdown(
  sourceText: string,
  options?: ScanEslintMarkdownOptions,
): Diagnostic[];

declare const api: {
  implementedEslintMarkdownRuleNames: typeof implementedEslintMarkdownRuleNames;
  scanEslintMarkdown: typeof scanEslintMarkdown;
};

export default api;

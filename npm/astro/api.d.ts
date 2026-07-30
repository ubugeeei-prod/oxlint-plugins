export type AstroRuleName =
  | 'no-deprecated-astro-canonicalurl'
  | 'no-deprecated-astro-fetchcontent'
  | 'no-deprecated-getentrybyslug';

export type AstroDiagnosticLoc = {
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
};

export type AstroDiagnosticFix = {
  /** UTF-8 byte offset into the original source. */
  start: number;
  /** UTF-8 byte offset into the original source. */
  end: number;
  replacement: string;
};

export type AstroDiagnostic = {
  ruleName: AstroRuleName;
  messageId: 'deprecated';
  loc: AstroDiagnosticLoc;
  fix?: AstroDiagnosticFix | null;
};

export type AstroScanOptions = {
  /** Empty or omitted means all implemented rules. */
  ruleNames?: readonly string[];
  /** Scan sourceText as an already extracted frontmatter segment. */
  frontmatterOnly?: boolean;
};

export function implementedAstroRuleNames(): AstroRuleName[];
export function scanAstro(
  sourceText: string,
  filename?: string,
  options?: AstroScanOptions,
): AstroDiagnostic[];

declare const api: {
  implementedAstroRuleNames: typeof implementedAstroRuleNames;
  scanAstro: typeof scanAstro;
};

export default api;

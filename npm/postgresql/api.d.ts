export type DiagnosticDatum = {
  key: string;
  value: string;
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
  ruleName: string;
  messageId: string;
  data: DiagnosticDatum[];
  loc: DiagnosticLoc;
  fix?: DiagnosticFix;
};

export type ScanPostgresqlOptions = {
  /** The rule names enabled for this scan. The adapter passes a single rule. */
  ruleNames?: readonly string[];
  /** The enabled rule's raw ESLint options array, verbatim. */
  options?: unknown;
};

export function implementedPostgresqlRuleNames(): string[];

export function scanPostgresql(sourceText: string, options?: ScanPostgresqlOptions): Diagnostic[];

declare const api: {
  implementedPostgresqlRuleNames: typeof implementedPostgresqlRuleNames;
  scanPostgresql: typeof scanPostgresql;
};

export default api;

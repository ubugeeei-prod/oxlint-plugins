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
  ruleName: 'no-unused-imports' | 'no-unused-vars';
  message: string;
  loc: DiagnosticLoc;
  fix?: DiagnosticFix;
};

export type ScanUnusedImportsOptions = {
  ruleNames?: readonly string[];
};

export function implementedUnusedImportsRuleNames(): Array<'no-unused-imports' | 'no-unused-vars'>;

export function scanUnusedImports(
  sourceText: string,
  filename?: string,
  options?: ScanUnusedImportsOptions,
): Diagnostic[];

declare const api: {
  implementedUnusedImportsRuleNames: typeof implementedUnusedImportsRuleNames;
  scanUnusedImports: typeof scanUnusedImports;
};

export default api;

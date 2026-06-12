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
  loc: DiagnosticLoc;
  fix?: DiagnosticFix;
};

export type SimpleImportSortScanOptions = {
  importGroups?: string[][];
};

export function implementedSimpleImportSortRuleNames(): string[];
export function scanSimpleImportSort(
  sourceText: string,
  filename?: string,
  options?: SimpleImportSortScanOptions,
): Diagnostic[];

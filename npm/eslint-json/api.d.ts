export type EslintJsonDiagnosticLoc = {
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
};

export type EslintJsonDiagnosticFix = {
  start: number;
  end: number;
  replacement: string;
};

export type EslintJsonDiagnosticData = {
  key?: string | null;
  value?: string | null;
  surrogate?: string | null;
  type?: string | null;
  thisName?: string | null;
  prevName?: string | null;
  direction?: string | null;
  sensitivity?: string | null;
  sortName?: string | null;
};

export type EslintJsonDiagnostic = {
  ruleName: string;
  messageId: string;
  data: EslintJsonDiagnosticData;
  loc: EslintJsonDiagnosticLoc;
  fix?: EslintJsonDiagnosticFix | null;
};

export type EslintJsonScanOptions = {
  ruleNames?: string[];
  normalizationForm?: 'NFC' | 'NFD' | 'NFKC' | 'NFKD';
  sortDirection?: 'asc' | 'desc' | 'ascending' | 'descending';
  sortCaseSensitive?: boolean;
  sortNatural?: boolean;
  sortMinKeys?: number;
  sortAllowLineSeparatedGroups?: boolean;
};

export function implementedEslintJsonRuleNames(): string[];
export function scanEslintJson(
  sourceText: string,
  options?: EslintJsonScanOptions,
): EslintJsonDiagnostic[];

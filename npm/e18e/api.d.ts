export interface E18eBanDependency {
  moduleName: string;
  messageId?: string;
  replacement?: string;
  url?: string;
  description?: string;
}

export interface E18eScanOptions {
  ruleNames?: string[];
  bannedDependencies?: E18eBanDependency[];
}

export interface DiagnosticData {
  array?: string;
  index?: string;
  item?: string;
  length?: string;
  value?: string;
  iterable?: string;
  mapper?: string;
  regex?: string;
  string?: string;
  original?: string;
  name?: string;
  replacement?: string;
  url?: string;
  description?: string;
}

export interface DiagnosticLoc {
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
}

export interface DiagnosticFix {
  start: number;
  end: number;
  replacement: string;
}

export interface Diagnostic {
  ruleName: string;
  messageId: string;
  data: DiagnosticData;
  loc: DiagnosticLoc;
  fix?: DiagnosticFix;
}

export function implementedE18eRuleNames(): string[];
export function scanE18e(
  sourceText: string,
  filename: string,
  options?: E18eScanOptions,
): Diagnostic[];

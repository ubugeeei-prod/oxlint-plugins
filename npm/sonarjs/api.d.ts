export interface SonarjsScanOptions {
  ruleNames?: string[];
}

export interface DiagnosticData {
  value?: string;
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

export function implementedSonarjsRuleNames(): string[];
export function scanSonarjs(
  sourceText: string,
  filename: string,
  options?: SonarjsScanOptions,
): Diagnostic[];

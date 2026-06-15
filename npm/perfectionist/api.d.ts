export type PerfectionistDiagnosticLoc = {
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
};

export type PerfectionistDiagnostic = {
  ruleName: string;
  messageId: string;
  loc: PerfectionistDiagnosticLoc;
};

export function implementedPerfectionistRuleNames(): string[];
export function scanPerfectionist(sourceText: string, filename?: string): PerfectionistDiagnostic[];

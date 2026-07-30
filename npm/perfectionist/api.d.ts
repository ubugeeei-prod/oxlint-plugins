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
  data?: {
    left: string;
    right: string;
    leftGroup?: string;
    rightGroup?: string;
  };
  fix?: {
    start: number;
    end: number;
    replacement: string;
  };
};

export function implementedPerfectionistRuleNames(): string[];
export function scanPerfectionist(sourceText: string, filename?: string): PerfectionistDiagnostic[];
export function scanPerfectionistRule(
  sourceText: string,
  filename?: string,
  ruleName?: string,
  options?: unknown[],
): PerfectionistDiagnostic[];

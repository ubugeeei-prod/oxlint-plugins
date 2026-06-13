export type RegexpDiagnosticData = {
  message?: string;
  flag?: string;
  flags?: string;
  sortedFlags?: string;
  expr?: string;
  charText?: string;
};

export type RegexpDiagnosticLoc = {
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
};

export type RegexpDiagnostic = {
  ruleName: string;
  messageId: string;
  data: RegexpDiagnosticData;
  loc: RegexpDiagnosticLoc;
};

export function implementedRegexpRuleNames(): string[];
export function scanRegexp(sourceText: string, filename?: string): RegexpDiagnostic[];

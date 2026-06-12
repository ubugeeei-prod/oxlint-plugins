export type SecurityDiagnosticLoc = {
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
};

export type SecurityDiagnosticData = {
  text?: string | null;
  method?: string | null;
  packageName?: string | null;
  fnName?: string | null;
  indices?: string | null;
  side?: string | null;
  value?: string | null;
  argumentType?: string | null;
};

export type SecurityDiagnostic = {
  ruleName: string;
  messageId: string;
  data: SecurityDiagnosticData;
  loc: SecurityDiagnosticLoc;
};

export function implementedSecurityRuleNames(): string[];
export function scanSecurity(sourceText: string, filename?: string): SecurityDiagnostic[];

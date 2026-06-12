export type CypressDiagnosticLoc = {
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
};

export type CypressDiagnosticFix = {
  start: number;
  end: number;
  replacement: string;
};

export type CypressDiagnostic = {
  ruleName: string;
  messageId: string;
  loc: CypressDiagnosticLoc;
  fix?: CypressDiagnosticFix | null;
};

export type CypressScanOptions = {
  unsafeToChainMethods?: string[];
};

export function implementedCypressRuleNames(): string[];
export function scanCypress(
  sourceText: string,
  filename?: string,
  options?: CypressScanOptions,
): CypressDiagnostic[];

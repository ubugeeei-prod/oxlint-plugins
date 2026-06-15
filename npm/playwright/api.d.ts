export type PlaywrightDiagnosticLoc = {
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
};

export type PlaywrightDiagnostic = {
  ruleName: string;
  messageId: string;
  loc: PlaywrightDiagnosticLoc;
};

export function implementedPlaywrightRuleNames(): string[];
export function scanPlaywright(sourceText: string, filename?: string): PlaywrightDiagnostic[];

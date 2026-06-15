export type AngularEslintDiagnosticLoc = {
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
};

export type AngularEslintDiagnostic = {
  ruleName: string;
  messageId: string;
  loc: AngularEslintDiagnosticLoc;
};

export function implementedAngularEslintRuleNames(): string[];
export function scanAngularEslint(sourceText: string, filename?: string): AngularEslintDiagnostic[];

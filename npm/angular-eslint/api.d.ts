export type AngularEslintDiagnosticLoc = {
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
};

export type AngularEslintDiagnostic = {
  ruleName: string;
  messageId: string;
  data: Array<{ key: string; value: string }>;
  loc: AngularEslintDiagnosticLoc;
};

export type NoInputRenameOptions = [
  {
    readonly allowedNames?: readonly string[];
  }?,
];

export type ConsistentComponentStylesOptions = [mode: 'array' | 'string'];

export type PreferSignalsOptions = [
  {
    readonly preferReadonlySignalProperties?: boolean;
    readonly preferInputSignals?: boolean;
    readonly preferQuerySignals?: boolean;
    readonly useTypeChecking?: boolean;
    readonly additionalSignalCreationFunctions?: readonly string[];
  }?,
];

export function implementedAngularEslintRuleNames(): string[];
export function scanAngularEslint(
  sourceText: string,
  filename?: string,
  options?: {
    ruleNames?: string[];
    options?: unknown[];
  },
): AngularEslintDiagnostic[];

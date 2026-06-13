export type FunctionalDiagnosticLoc = {
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
};

export type FunctionalDiagnostic = {
  ruleName: string;
  messageId: string;
  message: string;
  loc: FunctionalDiagnosticLoc;
};

export type FunctionalScanOptions = {
  ruleNames?: string[];
  allowRestParameter?: boolean;
  allowArgumentsKeyword?: boolean;
  allowLetInForLoopInit?: boolean;
  allowThrowToRejectPromises?: boolean;
  allowTryCatch?: boolean;
  allowTryFinally?: boolean;
  readonlyTypeMode?: 'generic' | 'keyword';
  ignoreIfReadonlyWrapped?: boolean;
};

export function implementedFunctionalRuleNames(): string[];
export function scanFunctional(
  sourceText: string,
  filename?: string,
  options?: FunctionalScanOptions,
): FunctionalDiagnostic[];

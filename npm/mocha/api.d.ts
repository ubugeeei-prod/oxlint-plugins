export type MochaDiagnosticLoc = {
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
};

export type MochaDiagnostic = {
  ruleName: string;
  message: string;
  loc: MochaDiagnosticLoc;
};

export type MochaScanOptions = {
  consistentInterface?: string;
  maxTopLevelSuitesLimit?: number;
  handleDoneIgnorePending?: boolean;
  noHooksAllowed?: string[];
  noHooksForSingleCaseAllowed?: string[];
  noSynchronousAllowed?: string[];
  noEmptyTitleMessage?: string;
  validSuiteTitlePattern?: string;
  validSuiteTitleMessage?: string;
  validTestTitlePattern?: string;
  validTestTitleMessage?: string;
  preferArrowAllowNamedFunctions?: boolean;
  preferArrowAllowUnboundThis?: boolean;
};

export function implementedMochaRuleNames(): string[];
export function scanMocha(
  sourceText: string,
  filename?: string,
  options?: MochaScanOptions,
): MochaDiagnostic[];

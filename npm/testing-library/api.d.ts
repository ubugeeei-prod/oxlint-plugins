export type TestingLibraryDiagnosticLoc = {
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
};

export type TestingLibraryDiagnostic = {
  ruleName: string;
  message: string;
  loc: TestingLibraryDiagnosticLoc;
};

export type TestingLibraryScanOptions = {
  ruleNames?: string[];
  testIdPattern?: string;
};

export function implementedTestingLibraryRuleNames(): string[];
export function scanTestingLibrary(
  sourceText: string,
  filename?: string,
  options?: TestingLibraryScanOptions,
): TestingLibraryDiagnostic[];

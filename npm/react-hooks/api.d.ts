export type ReactHooksDiagnosticLoc = {
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
};

export type ReactHooksDiagnosticData = {
  hook?: string | null;
  functionName?: string | null;
};

export type ReactHooksDiagnostic = {
  ruleName: string;
  messageId: string;
  data: ReactHooksDiagnosticData;
  loc: ReactHooksDiagnosticLoc;
};

export function implementedReactHooksRuleNames(): string[];
export function isHookName(name: string): boolean;
export function isReactComponentName(name: string): boolean;
export function scanReactHooks(sourceText: string, filename?: string): ReactHooksDiagnostic[];

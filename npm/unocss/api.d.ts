export type UnocssDiagnosticLoc = {
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
};

export type UnocssDiagnosticFix = {
  start: number;
  end: number;
  replacement: string;
};

export type UnocssDiagnostic = {
  ruleName: string;
  messageId: string;
  loc: UnocssDiagnosticLoc;
  fix?: UnocssDiagnosticFix | null;
  name?: string | null;
  reason?: string | null;
  prefix?: string | null;
};

export type UnocssBlocklistEntry = {
  name: string;
  reason?: string;
};

export type UnocssScanOptions = {
  unoFunctions?: string[];
  unoVariables?: string[];
  blocklist?: Array<string | UnocssBlocklistEntry | [string, { message?: string }]>;
  classCompilePrefix?: string;
  classCompileEnableFix?: boolean;
};

export function implementedUnocssRuleNames(): string[];
export function scanUnocss(
  sourceText: string,
  filename?: string,
  options?: UnocssScanOptions,
): UnocssDiagnostic[];

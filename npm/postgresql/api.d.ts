export type PostgresqlScanOptions = {
  identityStyle?: 'always' | 'never';
  jsonbStyle?: 'always' | 'never';
  textStyle?: 'always' | 'never';
  timestamptzStyle?: 'always' | 'never';
  notEqualsOperator?: '<>' | '!=';
  castForm?: 'operator' | 'function';
};

export type PostgresqlDiagnosticLoc = {
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
};

export type PostgresqlDiagnosticData = {
  op?: string | null;
  typeName?: string | null;
};

export type PostgresqlDiagnostic = {
  ruleName: string;
  messageId: string;
  data: PostgresqlDiagnosticData;
  loc: PostgresqlDiagnosticLoc;
};

export function implementedPostgresqlRuleNames(): string[];
export function scanPostgresql(
  sourceText: string,
  filename?: string,
  options?: PostgresqlScanOptions,
): PostgresqlDiagnostic[];

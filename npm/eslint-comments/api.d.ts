export type CommentKind = 'Line' | 'Block';

export interface CommentInput {
  kind: CommentKind;
  value: string;
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
}

export interface DiagnosticData {
  kind?: string;
  ruleId?: string;
  count?: number;
}

export interface DiagnosticLoc {
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
}

export interface Diagnostic {
  messageId: string;
  data: DiagnosticData;
  loc: DiagnosticLoc;
}

export declare function scanNoUnlimitedDisable(comments: CommentInput[]): Diagnostic[];

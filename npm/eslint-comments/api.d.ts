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

export interface PositionInput {
  line: number;
  column: number;
}

export interface ProblemInput {
  ruleId?: string | null;
  line?: number;
  column?: number;
  loc?: {
    start?: {
      line?: number;
      column?: number;
    };
  };
}

export declare function scanDisableEnablePair(
  comments: CommentInput[],
  allowWholeFile?: boolean,
  firstTokenStart?: PositionInput | null,
): Diagnostic[];
export declare function scanNoAggregatingEnable(comments: CommentInput[]): Diagnostic[];
export declare function scanNoDuplicateDisable(comments: CommentInput[]): Diagnostic[];
export declare function scanNoRestrictedDisable(
  comments: CommentInput[],
  patterns?: string[],
): Diagnostic[];
export declare function scanNoUnlimitedDisable(comments: CommentInput[]): Diagnostic[];
export declare function scanNoUnusedDisable(
  comments: CommentInput[],
  problems?: ProblemInput[],
): Diagnostic[];
export declare function scanNoUnusedEnable(comments: CommentInput[]): Diagnostic[];
export declare function scanNoUse(comments: CommentInput[], allow?: string[]): Diagnostic[];
export declare function scanRequireDescription(
  comments: CommentInput[],
  ignore?: string[],
): Diagnostic[];

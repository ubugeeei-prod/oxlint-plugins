export interface Position {
  line: number;
  column: number;
}

export interface SourceLocation {
  start: Position;
  end: Position;
}

export type Range = [number, number];

/** A `Program` node and the rest of the AST are loosely typed JSON. */
export type Node = {
  type: string;
  range?: Range;
  loc?: SourceLocation;
  [key: string]: unknown;
};

export interface ParseResult {
  ast: Node;
  visitorKeys: Record<string, string[]>;
  scopeManager: null;
}

/** Parse PostgreSQL SQL into an ESLint-compatible AST. */
export declare function parseForESLint(code: string): ParseResult;

/** Parse PostgreSQL SQL and return only the `Program` AST node. */
export declare function parse(code: string): Node;

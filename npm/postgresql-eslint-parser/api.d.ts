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

/** A PL function body embedded in a `CREATE FUNCTION` / `CREATE PROCEDURE`. */
export interface EmbeddedCode {
  type: 'EmbeddedCode';
  /** The lower-cased `LANGUAGE` clause (e.g. `"plv8"`, `"plpgsql"`). */
  language: string;
  /** The body text, as libpg_query reports it (escapes already resolved). */
  source: string;
  /** Whether the body was dollar-quoted (`$$`) or single-quoted (`'`). */
  quoteStyle: 'dollar' | 'single';
  /** Absolute UTF-16 offsets of the body contents within the SQL source. */
  range: Range;
  loc: SourceLocation;
}

/** Parse PostgreSQL SQL into an ESLint-compatible AST. */
export declare function parseForESLint(code: string): ParseResult;

/** Parse PostgreSQL SQL and return only the `Program` AST node. */
export declare function parse(code: string): Node;

/** Collect every `EmbeddedCode` node in a parsed program, in source order. */
export declare function extractEmbeddedCode(program: Node): EmbeddedCode[];

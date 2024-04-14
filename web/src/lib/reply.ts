import { Expr } from "./expr";

export interface NumberParts {
  exactValue: string | null;
  approxValue: string | null;
  factor: string | null;
  divfactor: string | null;
  rawUnit: Dimensionality | null;
  unit: string | null;
  quantity: string | null;
  dimensions: string | null;
  rawDimensions: Dimensionality | null;
}

export interface Dimensionality {
  [dimension: string]: number;
}

export interface NumberReply extends NumberParts {
  type: "number";
}

export interface DateReply {
  type: "date";
  year: number;
  month: number;
  day: number;
  hour: number;
  minute: number;
  second: number;
  nanosecond: number;
  string: string;
  rfc3339: string;
}

export interface PropertyReply {
  name: string;
  value: NumberParts;
  doc: string | null;
}

export interface SubstanceReply {
  type: "substance";
  name: string;
  doc: string | null;
  amount: NumberParts;
  properties: PropertyReply[];
}

export interface Duration {
  years: NumberParts;
  months: NumberParts;
  weeks: NumberParts;
  days: NumberParts;
  hours: NumberParts;
  minutes: NumberParts;
  seconds: NumberParts;
}

export interface DurationReply extends Duration {
  type: "duration";
  raw: NumberParts;
}

export interface ExprLiteral {
  type: "literal";
  text: string;
}

export interface ExprUnit {
  type: "unit";
  name: string;
}

export interface ExprProperty {
  type: "property";
  property: string;
  subject: ExprParts[];
}

export interface ExprError {
  type: "error";
  message: string;
}

export type ExprParts = ExprLiteral | ExprUnit | ExprProperty | ExprError;

export interface ExprReply {
  exprs: ExprParts[];
  ast: Expr;
}

export interface DefReply {
  type: "def";
  canonName: string;
  def: string | null;
  defExpr: ExprReply | null;
  value: NumberParts | null;
  doc: string | null;
}

export interface ConversionReply {
  type: "conversion";
  value: NumberParts;
}

export interface FactorizeReply {
  type: "factorize";
  factorizations: Dimensionality[];
}

export interface UnitsInCategory {
  category: string | null;
  units: string[];
}

export interface UnitsForReply {
  type: "unitsFor";
  units: UnitsInCategory[];
  of: NumberParts;
}

export interface UnitListReply {
  type: "unitList";
  rest: NumberParts;
  list: NumberParts[];
}

export interface SearchReply {
  type: "search";
  results: NumberParts[];
}

export type QueryReply =
  | NumberReply
  | DateReply
  | SubstanceReply
  | DurationReply
  | DefReply
  | ConversionReply
  | FactorizeReply
  | UnitsForReply
  | UnitListReply
  | SearchReply;

export interface ConformanceError {
  type: "conformance";
  left: NumberParts;
  right: NumberParts;
  suggestions: string[];
}

export interface NotFoundError {
  type: "notFound";
  got: string;
  suggestion: string | null;
}

export interface GenericError {
  type: "generic";
  message: string;
}

export type QueryError = ConformanceError | NotFoundError | GenericError;

export type Result<Ok, Err> =
  | ({ success: "ok" } & Ok)
  | ({ success: "err" } & Err);

export type QueryResult = Result<QueryReply, QueryError>;

export function describe(result: QueryResult): string {
  switch (result.type) {
    case "conformance":
      return "Conformance error";
    case "def":
      return `Definition of ${result.canonName}`;
    case "generic":
      return `Error: ${result.message}`;
    case "notFound":
      return `No such unit ${result.got}.`;
    case "search":
      return "Search results";
    case "substance":
      return `Definition of substance "${result.name}"`;
    case "unitsFor":
      return `Units for ${result.of.quantity}`;
    case "conversion":
    case "date":
    case "duration":
    case "factorize":
    case "number":
    case "unitList":
      return "";
  }
  return "";
}

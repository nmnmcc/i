export type AbiType = "nil" | "bool" | "int" | "float" | "string" | "bit_array" | "externref";
export type ErrorType = Exclude<AbiType, "externref">;
/** A function at the boundary: `(P1, …, Pn) -> A ? E` (Appendix B.2). */
export interface Signature {
  params: AbiType[];
  result: AbiType;
  /** The members of E in canonical order: nil, bool, int, float, string, bit_array. */
  errors?: ErrorType[];
  async?: boolean;
}
export interface EntryPoint { name: string; async?: boolean }
/** The ilang.abi metadata (Appendix B.6). */
export interface Abi {
  version: 2;
  entry?: EntryPoint | null;
  tests?: EntryPoint[];
  exports?: Record<string, Signature>;
  services?: Record<string, Record<string, Signature>>;
  externals?: Record<string, Record<string, Signature>>;
}
export interface BitArray { bytes: Uint8Array; bitLength: number }
/** nil: undefined or null; bool: boolean; int: bigint; float: number; string; bit_array: BitArray; externref: any. */
export type Value = undefined | null | boolean | bigint | number | string | BitArray | unknown;
export type ModuleSource = WebAssembly.Module | BufferSource | Response | Request | URL | string;
export interface Options {
  /** Metadata for a module without an ilang.abi section. */
  abi?: Abi;
  /**
   * Host service functions and external functions, by import module and name.
   * A service function throws `GuestError(value)` to give an error; an async
   * one gives a Promise.
   */
  imports?: Record<string, Record<string, (...args: any[]) => unknown>>;
  rawImports?: WebAssembly.Imports;
  /**
   * Check the answers of host functions in the loader, and stop the program
   * there, with no positions, when one is not correct. By default an answer
   * of the right type is given to the module as it is, and the module checks
   * it and stops with the positions of the call (B.2).
   */
  checkReplies?: boolean;
}
export interface Guest {
  readonly module: WebAssembly.Module;
  readonly instance: WebAssembly.Instance;
  readonly exports: WebAssembly.Exports;
  readonly abi: Readonly<Required<Abi>>;
  /** Why the instance cannot be used any more, or null. */
  readonly unusable: string | null;
  /** Gives the value; throws GuestError for an error of `? E` and Stop when the program stops. */
  call(name: string, args?: unknown[]): unknown;
  callAsync(name: string, args?: unknown[]): Promise<unknown>;
  /** Run the program entry; a Promise when it is async. */
  run(): void | Promise<void>;
  runEntryPoint(name: string): void | Promise<void>;
}
/** The program stopped. `stopMessage` is the stop message, or null for a trap without one. */
export class Stop extends Error {
  readonly stopMessage: string | null;
}
/** An error value of a function's `? E`. */
export class GuestError extends Error {
  constructor(value: unknown);
  readonly value: unknown;
}
export class AbiError extends Error {}
export const HOST_VALUE_REASON: "host: value is not correct";
export function supportsRequiredFeatures(): boolean;
export function integerToLimbs(value: bigint): { sign: number; bytes: Uint8Array };
export function integerFromLimbs(sign: number, bytes: Uint8Array): bigint;
export function readAbi(module: WebAssembly.Module, supplied?: Abi): Readonly<Required<Abi>>;
export function instantiate(source: ModuleSource, options?: Options): Promise<Guest>;
export function runTests(source: ModuleSource, options?: Options & { tests?: string[] }): Promise<Array<{ name: string; passed: boolean; error?: unknown }>>;

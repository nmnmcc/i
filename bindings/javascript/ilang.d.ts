export type AbiType = "nil" | "bool" | "float" | "int" | "string" | "bit_array" | "externref";
export interface Signature { params: AbiType[]; result: AbiType }
export interface Abi {
  version: 1;
  exports?: Record<string, Signature>;
  imports?: Record<string, Record<string, Signature>>;
  entry?: string | null;
  tests?: string[];
}
export interface BitArray { bytes: Uint8Array; bitLength: number }
export type ModuleSource = WebAssembly.Module | BufferSource | Response | Request | URL | string;
export interface Options {
  abi?: Abi;
  imports?: Record<string, Record<string, (...args: unknown[]) => unknown>>;
  rawImports?: WebAssembly.Imports;
  stdout?: (text: string) => void;
  stderr?: (text: string) => void;
  random?: (bytes: Uint8Array) => void;
}
export interface Guest {
  readonly module: WebAssembly.Module;
  readonly instance: WebAssembly.Instance;
  readonly exports: WebAssembly.Exports;
  readonly abi: Abi;
  call(name: string, args?: unknown[]): unknown;
  run(): void;
}
export class iLangPanic extends Error {}
export class iLangAbiError extends Error {}
export function supportsRequiredFeatures(): boolean;
export function integerToLimbs(value: bigint): { sign: number; bytes: Uint8Array };
export function integerFromLimbs(sign: number, bytes: Uint8Array): bigint;
export function readAbi(module: WebAssembly.Module, supplied?: Abi): Abi;
export function instantiate(source: ModuleSource, options?: Options): Promise<Guest>;
export function runTests(source: ModuleSource, options?: Options & { tests?: string[] }): Promise<Array<{ name: string; passed: boolean; error?: unknown }>>;

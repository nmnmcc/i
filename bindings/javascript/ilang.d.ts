export type AbiType = "nil" | "bool" | "int" | "float" | "string" | "bit_array" | "externref";
export type ErrorType = Exclude<AbiType, "externref">;
/** A function at the boundary: `(P1, …, Pn) -> A ? E`. */
export interface Signature {
  params: AbiType[];
  result: AbiType;
  /** The members of E in canonical order: nil, bool, int, float, string, bit_array. */
  errors?: ErrorType[];
  async?: boolean;
}
export interface EntryPoint { name: string; async?: boolean }
/** The ilang.abi metadata. */
export interface Abi {
  version: 4;
  entry?: EntryPoint | null;
  tests?: EntryPoint[];
  exports?: Record<string, Signature>;
  services?: Record<string, Record<string, Signature>>;
  externals?: Record<string, Record<string, Signature>>;
}
export interface SourcePosition { path: string; start: number; end: number; line: number; column: number }
export interface CollectionEntry {
  handle: string; collection: string; signature: string; data: [string, string][];
  target: SourcePosition; at: SourcePosition;
  calls: { macro_name: string; position: SourcePosition }[];
}
export interface CollectionArtifact {
  manifest: { revision: string; entries: CollectionEntry[] };
  selected: string; export: string; is_async: boolean;
}
export function readCollections(module: WebAssembly.Module): CollectionArtifact | null;
export interface BitArray { bytes: Uint8Array; bitLength: number }
/** nil: undefined or null; bool: boolean; int: bigint; float: number; string; bit_array: BitArray; externref: any. */
export type Value = undefined | null | boolean | bigint | number | string | BitArray | unknown;
export type ModuleSource = WebAssembly.Module | BufferSource | Response | Request | URL | string;
export interface Options {
  /** Installs the standard IO services; args are passed to i/process. */
  standardServices?: boolean | { args?: string[] };
  /** Context registrations are restricted to approved resolved declarations. */
  contextImports?: Record<string, Record<string, (context: ResourceContext, ...args: any[]) => unknown>>;
  /** Host approval from authorizeBindings; a plain manifest cannot grant itself. */
  bindings?: BindingAuthorization;
  /** Reuses this host across instances/tests; each invocation has its own table. */
  resourceHost?: ResourceHost;
  /** Metadata for a module without an ilang.abi section. */
  abi?: Abi;
  /**
   * Host service functions and external functions, by import module and name.
   * A service function throws `GuestError(value)` to give an error; an async
   * one gives a Promise. An async one is called during the import; it may
   * call `requestSignal()` before its first `await`.
   */
  imports?: Record<string, Record<string, (...args: any[]) => unknown>>;
  rawImports?: WebAssembly.Imports;
  /**
   * Checks the answers of host functions in the loader, and stops the program
   * there, with no positions, when one is not correct. By default an answer
   * of the right type is given to the module as it is, and the module checks
   * it and stops with the positions of the call.
   */
  checkReplies?: boolean;
}
export interface Guest {
  readonly module: WebAssembly.Module;
  readonly instance: WebAssembly.Instance;
  readonly exports: WebAssembly.Exports;
  readonly abi: Readonly<Required<Abi>>;
  readonly collections: CollectionArtifact | null;
  /** Why the instance cannot be used any more, or null. */
  readonly unusable: string | null;
  /** Stops admission and cancels work/resources independently of language finals. */
  dispose(): void;
  /** Drains the host-wide cleanup diagnostic queue without entering Wasm. */
  takeCleanupErrors(): CleanupFailure[];
  /** Gives the value; throws GuestError for an error of `? E` and Stop when the program stops. */
  call(name: string, args?: unknown[]): unknown;
  callAsync(name: string, args?: unknown[]): Promise<unknown>;
  /** Runs the program entry; a Promise when it is async. */
  run(): void | Promise<void>;
  runEntryPoint(name: string): void | Promise<void>;
  runCollection(handle: string): void | Promise<void>;
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
/**
 * The AbortSignal of the async request whose service is starting; it is
 * aborted when the request is dropped. Throws when no service is starting.
 */
export function requestSignal(): AbortSignal;
export function readAbi(module: WebAssembly.Module, supplied?: Abi): Readonly<Required<Abi>>;
export function instantiate(source: ModuleSource, options?: Options): Promise<Guest>;
export function runTests(source: ModuleSource, options?: Options & { tests?: string[] }): Promise<Array<{ name: string; passed: boolean; error?: unknown }>>;

/** Required resolved declaration before Wasm imports are coalesced. */
export interface BindingDeclaration {
  package: string;
  module: string;
  module_digest: string;
  declaration: string;
  namespace: string;
  name: string;
  kind: "service" | "external";
  signature: Signature;
}
export interface BindingPlan {
  artifact_digest: string;
  declarations: BindingDeclaration[];
}
/** Opaque host approval tied to a module compiled from exact artifact bytes. */
export interface BindingAuthorization { readonly module: WebAssembly.Module }
export function authorizeBindings(source: Exclude<ModuleSource, WebAssembly.Module>, plan: BindingPlan,
  approve: (declaration: Readonly<BindingDeclaration>) => boolean | Promise<boolean>, options?: { abi?: Abi }): Promise<BindingAuthorization>;
export class ResourceError extends Error { readonly code: "closed" | "busy" | "memory exhausted" }
export interface CleanupFailure { readonly phase: "cancel" | "wake" | "cleanup"; readonly error: unknown }
export class ResourceHost {
  constructor(options?: { limit?: bigint });
  context(): ResourceContext;
  takeCleanupErrors(): CleanupFailure[];
}
export interface ResourceLease<T> {
  readonly value: T;
  readonly signal: AbortSignal;
  release(): void;
  commit<A>(answer: A): A;
}
export interface ProvisionalResource {
  readonly id: bigint;
  deliver(): bigint;
  discard(): Promise<void>;
}
export class ResourceContext {
  private constructor();
  readonly live: boolean;
  abortController(phase?: "cancel" | "wake"): AbortController;
  ownRequest(cancel: () => void): () => void;
  cleanupJob(): { readonly receipt: Promise<void>; complete(): void };
  insert<T>(kind: string, value: T, cleanup: (value: T) => void | Promise<void>): bigint;
  acquire<T = unknown>(id: bigint, kind: string, lane: string): ResourceLease<T>;
  takeIdle<T = unknown>(id: bigint, kind: string): T;
  close(id: bigint, kind: string): Promise<void>;
  teardown(): Promise<void>;
  provisional(id: bigint, kind: string): ProvisionalResource;
}

/** Constructs context-aware standard service registrations. */
export function standardServices(options?: { args?: string[] }): Record<string, Record<string, (context: ResourceContext, ...args: any[]) => unknown>>;

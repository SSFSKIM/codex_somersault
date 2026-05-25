// Ambient declarations for build-time-injected globals that Anthropic's
// `bun build` config substituted at compile time. These are documented as
// unfalsifiable from the leak alone (docs/specs/00-overview.md §13.4,
// 22-service-api.md §12 Q2: "bundler-injected globals; values not source-
// derivable"). Somersault declares them as ambient with sensible stub values
// so the typecheck passes; the actual runtime values can be threaded later.

declare const MACRO: {
  /** Feedback channel URL fragment used in error messages (e.g. tool-use
   *  mismatch hints in services/api/errors.ts). */
  readonly FEEDBACK_CHANNEL: string
  /** ISO timestamp of the build (used in /doctor and About surfaces). */
  readonly BUILD_TIME: string
  /** Allow additional fields rather than maintaining a strict schema —
   *  the leaked tree references MACRO.X at multiple call sites; if a
   *  new field appears in upstream, falling back to `any` here avoids a
   *  cascade of typecheck breaks. */
  readonly [key: string]: string
}

declare const Bun: any
declare const GateOverridesWarning: any
declare const ExperimentEnrollmentNotice: any
declare const HOOK_TIMING_DISPLAY_THRESHOLD_MS: number
declare const TungstenPill: any
declare const Gates: any
declare const apiMetricsRef: any
declare const computeTtftText: any
declare const getAntModelOverrideConfig: any
declare const resolveAntModel: any
declare const getAntModels: any
declare const fireCompanionObserver: any
declare const UltraplanChoiceDialog: any
declare const UltraplanLaunchDialog: any
declare const launchUltraplan: any
type T = any

interface ObjectConstructor {
  values(o: any): any[]
  entries(o: any): [string, any][]
  keys(o: any): string[]
  fromEntries(entries: Iterable<readonly any[]>): any
}

interface PromiseConstructor {
  race(values: any): Promise<any>
}

type ErrnoException = NodeJS.ErrnoException
type PromiseWithResolvers<T> = {
  promise: Promise<T>
  resolve: (value: T | PromiseLike<T>) => void
  reject: (reason?: any) => void
}

declare namespace NodeJS {
  interface ProcessEnv {
    USER_TYPE?: string
    NODE_ENV?: string
  }
}

declare namespace JSX {
  type Element = any
  interface ElementClass {
    render?: any
  }
  interface IntrinsicAttributes {
    [key: string]: any
  }
  interface IntrinsicElements {
    [elemName: string]: any
  }
}

declare module 'react/compiler-runtime' {
  export function c(size: number): any[]
}

declare module 'bun:ffi' {
  export const dlopen: any
  export const FFIType: any
  export const suffix: any
  export const ptr: any
  export const CString: any
}

declare module '*.md' {
  const content: string
  export default content
}

declare module 'p-map' {
  const pMap: any
  export default pMap
}

declare module 'fuse.js' {
  class Fuse<T = any> {
    constructor(...args: any[])
    [key: string]: any
  }
  export default Fuse
}

declare module 'xss' {
  const xss: any
  export default xss
}

declare module 'asciichart' {
  export const asciichart: any
  export const plot: any
  export default asciichart
}

declare module 'color-diff-napi' {
  export class ColorDiff {
    constructor(...args: any[])
    [key: string]: any
  }
  export class ColorFile {
    constructor(...args: any[])
    [key: string]: any
  }
  export type SyntaxTheme = any
  export const getSyntaxTheme: any
  export const nativeGetSyntaxTheme: any
}

declare module 'env-paths' {
  const envPaths: any
  export default envPaths
}

declare module 'picomatch' {
  const picomatch: any
  export default picomatch
}

declare module 'supports-hyperlinks' {
  const supportsHyperlinks: any
  export default supportsHyperlinks
}

declare module 'tree-kill' {
  const treeKill: any
  export default treeKill
}

declare module 'turndown' {
  class TurndownService {
    constructor(...args: any[])
    [key: string]: any
  }
  export default TurndownService
}

declare module 'ignore' {
  function ignore(...args: any[]): any
  export default ignore
}

declare module 'proper-lockfile' {
  export type CheckOptions = any
  export type LockOptions = any
  export type UnlockOptions = any
  export const check: any
  export const lock: any
  export const lockSync: any
  export const unlock: any
}

declare module 'shell-quote' {
  export type ControlOperator = any
  export type ParseEntry = any
  export const parse: any
  export const quote: any
}

declare module 'highlight.js' {
  const hljs: any
  export const getLanguage: any
  export const highlight: any
  export default hljs
}

declare module 'cli-highlight' {
  export const highlight: any
  export const supportsLanguage: any
  export const listLanguages: any
}

declare module 'sharp' {
  const sharp: any
  export default sharp
}

declare module 'image-processor-napi' {
  export const getImageDimensions: any
  export const resizeImage: any
  export const encodeImageToBase64: any
  export const getNativeModule: any
  export const sharp: any
  const imageProcessor: any
  export default imageProcessor
}

declare module 'audio-capture-napi' {
  export const isNativeAudioAvailable: any
  export const isNativeRecordingActive: any
  export const startNativeRecording: any
  export const stopNativeRecording: any
  const audioCapture: any
  export default audioCapture
}

declare module 'url-handler-napi' {
  export const waitForUrlEvent: any
  const urlHandler: any
  export default urlHandler
}

declare module 'jsonc-parser/lib/esm/main.js' {
  export const applyEdits: any
  export const modify: any
  export const parse: any
}

declare module 'vscode-jsonrpc/node.js' {
  export type MessageConnection = any
  export class StreamMessageReader {
    constructor(...args: any[])
  }
  export class StreamMessageWriter {
    constructor(...args: any[])
  }
  export const Trace: any
  export const createMessageConnection: any
}

declare module 'vscode-languageserver-protocol' {
  export type InitializeParams = any
  export type InitializeResult = any
  export type PublishDiagnosticsParams = any
  export type ServerCapabilities = any
}

declare module 'vscode-languageserver-types' {
  export type CallHierarchyIncomingCall = any
  export type CallHierarchyItem = any
  export type CallHierarchyOutgoingCall = any
  export type DocumentSymbol = any
  export type Hover = any
  export type Location = any
  export type LocationLink = any
  export type MarkedString = any
  export type MarkupContent = any
  export type SymbolInformation = any
  export enum SymbolKind {
    File = 1,
  }
}

declare module '@commander-js/extra-typings' {
  export class Command {
    constructor(...args: any[])
    [key: string]: any
  }
  export class InvalidArgumentError extends Error {
    constructor(...args: any[])
  }
  export class Option {
    constructor(...args: any[])
    [key: string]: any
  }
}

declare module '@growthbook/growthbook' {
  export class GrowthBook {
    constructor(...args: any[])
    [key: string]: any
  }
}

declare module '@anthropic-ai/claude-agent-sdk' {
  export type PermissionMode = any
}

declare module '@anthropic-ai/mcpb' {
  export type McpbManifest = any
  export type McpbUserConfigurationOption = any
  export const extractMcpb: any
  export const getMcpConfigForManifest: any
  export const McpbManifestSchema: any
  export const validateMcpb: any
  const mcpb: any
  export default mcpb
}

declare module '@anthropic-ai/sandbox-runtime' {
  export type FsReadRestrictionConfig = any
  export type FsWriteRestrictionConfig = any
  export type IgnoreViolationsConfig = any
  export type NetworkHostPattern = any
  export type NetworkRestrictionConfig = any
  export type SandboxAskCallback = any
  export type SandboxDependencyCheck = any
  export const SandboxManager: any
  export type SandboxRuntime = any
  export type SandboxRuntimeConfig = any
  export type SandboxRuntimeFactory = any
  export type SandboxSession = any
  export type SandboxSessionConfig = any
  export type SandboxSessionStatus = any
  export type SandboxSnapshot = any
  export type SandboxViolationEvent = any
  export type SandboxViolationStore = any
  export const SandboxViolationStore: any
  export const SandboxRuntimeConfigSchema: any
  export const createSandboxRuntime: any
}

declare module '@ant/claude-for-chrome-mcp' {
  export const BROWSER_TOOLS: any
  export type ClaudeForChromeContext = any
  export type Logger = any
  export type PermissionMode = any
  export const createClaudeForChromeMcpServer: any
}

declare module '@ant/computer-use-mcp' {
  export const API_RESIZE_PARAMS: any
  export const bindSessionContext: any
  export const buildComputerUseTools: any
  export const ComputerExecutor: any
  export type ComputerExecutor = any
  export type ComputerUseSessionContext = any
  export const createComputerUseMcpServer: any
  export type CuCallToolResult = any
  export type CuPermissionRequest = any
  export type CuPermissionResponse = any
  export const DEFAULT_GRANT_FLAGS: any
  export type DisplayGeometry = any
  export type FrontmostApp = any
  export type InstalledApp = any
  export type ResolvePrepareCaptureResult = any
  export type RunningApp = any
  export type ScreenshotDims = any
  export type ScreenshotResult = any
  export const targetImageSize: any
  export type ComputerUseMcpServer = any
  export type ComputerUseTool = any
  export type ComputerUseToolName = any
}

declare module '@ant/computer-use-mcp/types' {
  export type ComputerUseHostAdapter = any
  export type CoordinateMode = any
  export type CuPermissionRequest = any
  export type CuPermissionResponse = any
  export type CuSubGates = any
  export const DEFAULT_GRANT_FLAGS: any
  export type HostApp = any
  export type HostAppState = any
  export type ImageSnapshot = any
  export type Logger = any
  export type SentinelApp = any
  export type SentinelAppConfig = any
}

declare module '@ant/computer-use-mcp/sentinelApps' {
  export const getSentinelCategory: any
  export const sentinelApps: any
}

declare module '@ant/computer-use-input' {
  export type ComputerUseInput = any
  export type ComputerUseInputAPI = any
  const computerUseInput: any
  export default computerUseInput
}

declare module '@ant/computer-use-swift' {
  export type ComputerUseAPI = any
  const computerUseSwift: any
  export default computerUseSwift
}

declare module '@aws-sdk/client-bedrock' {
  export const BedrockClient: any
  export const GetInferenceProfileCommand: any
  export const ListInferenceProfilesCommand: any
  export const ListFoundationModelsCommand: any
}

declare module '@aws-sdk/client-bedrock-runtime' {
  export type CountTokensCommandInput = any
  export const BedrockRuntimeClient: any
  export const CountTokensCommand: any
}

declare module '@aws-sdk/client-sts' {
  export const STSClient: any
  export const GetCallerIdentityCommand: any
}

declare module '@aws-sdk/credential-providers' {
  export const fromIni: any
  export const fromNodeProviderChain: any
}

declare module '@aws-sdk/credential-provider-node' {
  export const defaultProvider: any
}

declare module '@smithy/node-http-handler' {
  export const NodeHttpHandler: any
}

declare module '@smithy/core' {
  export const getDefaultRoleAssumerWithWebIdentity: any
  export const NoAuthSigner: any
}

declare module 'undici' {
  export type Dispatcher = any
  export const Agent: any
  export class EnvHttpProxyAgent {
    constructor(...args: any[])
  }
  export namespace EnvHttpProxyAgent {
    export type Options = any
  }
  export const ProxyAgent: any
  export const setGlobalDispatcher: any
}

declare module 'cacache' {
  export const ls: any
  export const rm: any
  const cacache: any
  export default cacache
}

declare module '@opentelemetry/exporter-metrics-otlp-grpc' {
  export const OTLPMetricExporter: any
}

declare module '@opentelemetry/exporter-metrics-otlp-http' {
  export const OTLPMetricExporter: any
}

declare module '@opentelemetry/exporter-metrics-otlp-proto' {
  export const OTLPMetricExporter: any
}

declare module '@opentelemetry/exporter-prometheus' {
  export const PrometheusExporter: any
}

declare module '@opentelemetry/exporter-logs-otlp-grpc' {
  export const OTLPLogExporter: any
}

declare module '@opentelemetry/exporter-logs-otlp-http' {
  export const OTLPLogExporter: any
}

declare module '@opentelemetry/exporter-logs-otlp-proto' {
  export const OTLPLogExporter: any
}

declare module '@opentelemetry/exporter-trace-otlp-grpc' {
  export const OTLPTraceExporter: any
}

declare module '@opentelemetry/exporter-trace-otlp-http' {
  export const OTLPTraceExporter: any
}

declare module '@opentelemetry/exporter-trace-otlp-proto' {
  export const OTLPTraceExporter: any
}

declare module 'plist' {
  export const parse: any
  const plist: any
  export default plist
}

declare module '@azure/identity' {
  export const AzureCliCredential: any
  export const DefaultAzureCredential: any
  export const getBearerTokenProvider: any
}

declare module '@anthropic-ai/bedrock-sdk' {
  export const AnthropicBedrock: any
  const BedrockSDK: any
  export default BedrockSDK
}

declare module '@anthropic-ai/foundry-sdk' {
  export const AnthropicFoundry: any
  const FoundrySDK: any
  export default FoundrySDK
}

declare module '@anthropic-ai/vertex-sdk' {
  export const AnthropicVertex: any
  const VertexSDK: any
  export default VertexSDK
}

declare module 'react' {
  export type ReactNode = any
  export type ReactElement = any
  export type ComponentType<P = any> = any
  export type FC<P = any> = any
  export type PropsWithChildren<P = any> = P & { children?: ReactNode }
  export type Ref<T = any> = any
  export type RefObject<T = any> = { current: T }
  export type MutableRefObject<T = any> = { current: T }
  export type Dispatch<A = any> = (value: A) => void
  export type SetStateAction<S = any> = S | ((prevState: S) => S)
  export type CSSProperties = any
  export type KeyboardEvent<T = any> = any
  export type MouseEvent<T = any> = any

  export const Fragment: any
  export const Suspense: any
  export const Children: any
  export class Component<P = any, S = any, SS = any> {
    props: P
    state: S
    context: any
    refs: any
    constructor(props: P)
    setState(...args: any[]): void
    forceUpdate(...args: any[]): void
    componentDidMount?(): void
    componentDidUpdate?(prevProps: P, prevState: S, snapshot?: SS): void
    componentWillUnmount?(): void
    componentDidCatch?(error: Error, errorInfo?: any): void
    render(): ReactNode
  }
  export class PureComponent<P = any, S = any, SS = any> extends Component<P, S, SS> {}
  export const createElement: any
  export const cloneElement: any
  export function isValidElement<P = any>(object: any): object is { props: P; key?: any; type?: any }
  export const memo: any
  export const lazy: any
  export const forwardRef: any
  export function createContext<T = any>(defaultValue?: T): any
  export function useState(initialState: []): [any[], Dispatch<any>]
  export function useState<S = any>(initialState?: S | (() => S)): [any, Dispatch<any>]
  export function useEffect(effect: (...args: any[]) => any, deps?: any[]): void
  export function useLayoutEffect(effect: (...args: any[]) => any, deps?: any[]): void
  export function useMemo<T = any>(factory: () => T, deps?: any[]): any
  export function useCallback<T extends (...args: any[]) => any>(callback: T, deps?: any[]): T
  export function useRef<T = any>(initialValue?: T): MutableRefObject<any>
  export function useContext<T = any>(context: any): T
  export function useReducer<R = any>(...args: any[]): any
  export function useImperativeHandle<T = any, R = any>(...args: any[]): any
  export function useDeferredValue<T = any>(value: T): T
  export function useInsertionEffect(effect: (...args: any[]) => any, deps?: any[]): void
  export function useSyncExternalStore<T = any>(...args: any[]): T
  export function use<T>(usable: Promise<T> | { then?: any } | T): T
  export function useEffectEvent<T extends (...args: any[]) => any>(fn: T): T

  const React: {
    Fragment: any
    Suspense: any
    Children: any
    Component: typeof Component
    PureComponent: typeof PureComponent
    createElement: any
    cloneElement: any
    isValidElement: typeof isValidElement
    memo: any
    lazy: any
    forwardRef: any
    createContext: typeof createContext
    useState: typeof useState
    useEffect: typeof useEffect
    useLayoutEffect: typeof useLayoutEffect
    useMemo: typeof useMemo
    useCallback: typeof useCallback
    useRef: typeof useRef
    useContext: typeof useContext
    useReducer: (...args: any[]) => any
    useImperativeHandle: (...args: any[]) => any
    useDeferredValue: typeof useDeferredValue
    useInsertionEffect: typeof useInsertionEffect
    useSyncExternalStore: (...args: any[]) => any
    use: typeof use
    useEffectEvent: typeof useEffectEvent
  }
  export default React
}

declare module 'diff' {
  export type StructuredPatchHunk = any
  export const diffArrays: any
  export const diffLines: any
  export const diffWordsWithSpace: any
  export const structuredPatch: any
  export const parsePatch: any
  export const createPatch: any
  export const applyPatch: any
}

declare module 'qrcode' {
  export function toString(text: any, options?: any): Promise<string>
}

declare module 'react-reconciler/constants.js' {
  export const ConcurrentRoot: any
  export const ContinuousEventPriority: any
  export const DefaultEventPriority: any
  export const DiscreteEventPriority: any
  export const LegacyRoot: any
  export const NoEventPriority: any
}

declare module '@opentelemetry/resources' {
  export const envDetector: any
  export const hostDetector: any
  export const osDetector: any
  export const resourceFromAttributes: any
}

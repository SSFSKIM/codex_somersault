// Stub for the missing-from-leak constants/querySource module.
// Per spec 04 §3.1 / §5.10 / §5.12 etc., QuerySource is a string-literal
// union covering 'repl_main_thread' (with variants), 'sdk', 'agent:*',
// 'compact', 'hook_agent', 'verification_agent', 'side_question',
// 'auto_mode', 'bash_classifier', 'session_memory', 'agent_summary', etc.
// Stubbed as `string` for typecheck; Phase 2 can author the precise union.

export type QuerySource = string

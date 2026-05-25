// ChatGPT subscription OAuth for OpenAI access — adapted from hermes-agent
// (NousResearch/hermes-agent, hermes_cli/auth.py).
//
// Idea: instead of paying per-token via `api.openai.com`, route through
// `chatgpt.com/backend-api/codex` using the same OAuth tokens the official
// `codex` CLI mints. Billing then falls under the user's ChatGPT Plus/Pro
// subscription (flat monthly fee).
//
// Strategy: we READ ~/.codex/auth.json directly on every request — no
// separate Somersault store. Earlier versions cached tokens in
// ~/.somersault/codex-auth.json to "avoid refresh-token rotation conflicts"
// (per hermes' note), but that pattern caused the opposite problem: when
// Codex CLI refreshed tokens, our cached copy went stale and the server
// invalidated our old access_token with HTTP 401 "authentication token has
// been invalidated". Reading the live file means Codex CLI's refresh
// cadence always wins, with zero conflict.
//
// We do NOT initiate our own refresh — let the `codex` CLI maintain
// freshness. If `~/.codex/auth.json` ever has a truly-stale token, the
// user runs `codex login` (or any `codex` command, which refreshes
// transparently) and we pick up the new token on the next invocation.

import { existsSync, readFileSync } from 'fs'
import { homedir } from 'os'
import { join } from 'path'

const CODEX_CLI_AUTH_PATH = join(homedir(), '.codex', 'auth.json')

// Inference endpoint for ChatGPT-subscription billing.
export const CODEX_BACKEND_BASE_URL = 'https://chatgpt.com/backend-api/codex'

type CodexTokens = {
  access_token: string
  refresh_token?: string
  id_token?: string
  account_id?: string
}

type CodexAuthFile = {
  tokens: CodexTokens
  auth_mode?: 'chatgpt' | 'apikey'
  last_refresh?: string
  OPENAI_API_KEY?: string
}

export type CodexCredential = {
  accessToken: string
  baseURL: string
  accountId?: string
}

/**
 * Returns Codex OAuth credentials if available, else null. Caller should
 * fall through to API-key auth or surface a configuration error.
 *
 * Always reads ~/.codex/auth.json directly so we follow Codex CLI's token
 * rotation in real time. No caching — the file read is cheap and the
 * server invalidates the older access_token whenever Codex CLI refreshes.
 */
export async function getCodexCredential(): Promise<CodexCredential | null> {
  const state = readCodexCliAuth()
  if (!state) return null
  return {
    accessToken: state.tokens.access_token,
    baseURL: CODEX_BACKEND_BASE_URL,
    accountId: state.tokens.account_id,
  }
}

function readCodexCliAuth(): CodexAuthFile | null {
  if (!existsSync(CODEX_CLI_AUTH_PATH)) return null
  try {
    const raw = JSON.parse(readFileSync(CODEX_CLI_AUTH_PATH, 'utf8'))
    // Codex CLI sets auth_mode to 'apikey' when the user is on the
    // API-key path; only ChatGPT-subscription mode is useful here.
    if (raw.auth_mode && raw.auth_mode !== 'chatgpt') return null
    if (!isValidState(raw)) return null
    return raw as CodexAuthFile
  } catch {
    return null
  }
}

function isValidState(value: unknown): value is CodexAuthFile {
  if (!value || typeof value !== 'object') return false
  const v = value as Record<string, unknown>
  const tokens = v.tokens
  if (!tokens || typeof tokens !== 'object') return false
  const t = tokens as Record<string, unknown>
  return typeof t.access_token === 'string' && t.access_token.length > 0
}

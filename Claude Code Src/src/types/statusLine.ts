export type StatusLineCommandInput = {
  session_id: string
  transcript_path: string
  cwd: string
  permission_mode?: string
  session_name?: string
  model?: {
    id: string
    display_name: string
    [key: string]: any
  }
  workspace?: {
    current_dir: string
    project_dir: string
    added_dirs: string[]
    [key: string]: any
  }
  version?: string
  output_style?: {
    name: string
    [key: string]: any
  }
  cost?: Record<string, any>
  context_window?: Record<string, any>
  exceeds_200k_tokens?: boolean
  rate_limits?: {
    five_hour?: {
      used_percentage: number
      resets_at?: string | number
    }
    seven_day?: {
      used_percentage: number
      resets_at?: string | number
    }
    [key: string]: any
  }
  vim?: {
    mode: string
  }
  agent?: {
    name: string
  }
  remote?: {
    session_id: string
  }
  worktree?: Record<string, any>
  agent_id?: string
  agent_type?: string
  [key: string]: any
}

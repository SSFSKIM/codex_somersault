// Content for the verify bundled skill.
// Each .md file is inlined as a string at build time via Bun's text loader.

import cliMd from './verify/examples/cli.md' with { type: 'text' }
import serverMd from './verify/examples/server.md' with { type: 'text' }
import skillMd from './verify/SKILL.md' with { type: 'text' }

export const SKILL_MD: string = skillMd

export const SKILL_FILES: Record<string, string> = {
  'examples/cli.md': cliMd,
  'examples/server.md': serverMd,
}

export type ViewState = any

export type ParentViewState = ViewState

export type PluginSettingsProps = {
  onDone?: (message?: string | null) => void
  [key: string]: any
}

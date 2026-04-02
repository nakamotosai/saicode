export type ViewState =
  | { type: 'menu' }
  | { type: string; [key: string]: unknown }

export type PluginSettingsProps = {
  onDone?: (message?: string | null) => void
  targetMarketplace?: string
  targetPlugin?: string
  action?: string
}

export type PluginRow = Record<string, unknown>
export type PluginInstallState = Record<string, unknown>
export type PluginAction = string

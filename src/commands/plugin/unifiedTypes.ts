export type UnifiedInstalledItem = {
  id?: string
  name?: string
  type?: string
  scope?: string
  plugin?: any
  [key: string]: unknown
}

export type UnifiedPluginMetadata = Record<string, unknown>
export type UnifiedPluginVersion = Record<string, unknown>


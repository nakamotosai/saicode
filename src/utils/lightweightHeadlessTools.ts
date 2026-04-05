export const SIMPLE_TOOL_NAMES = new Set([
  'Bash',
  'Glob',
  'Grep',
  'Read',
  'Edit',
  'Write',
])

export const LIGHTWEIGHT_HEADLESS_TOOL_NAMES = new Set([
  ...SIMPLE_TOOL_NAMES,
  'WebFetch',
  'WebSearch',
])

export function normalizeToolRestrictionValues(values: string[]): string[] {
  return values
    .flatMap(value => value.split(/[,\s]+/))
    .map(value => value.trim())
    .filter(Boolean)
    .map(value => value.replace(/\(.*$/, ''))
}

function usesOnlyToolNamesFromSet(
  values: string[],
  allowedSet: ReadonlySet<string>,
): boolean {
  const normalized = normalizeToolRestrictionValues(values)
  return (
    normalized.length > 0 &&
    normalized.every(value => allowedSet.has(value))
  )
}

export function usesOnlySimpleTools(values: string[]): boolean {
  return usesOnlyToolNamesFromSet(values, SIMPLE_TOOL_NAMES)
}

export function usesOnlyLightweightHeadlessTools(values: string[]): boolean {
  return usesOnlyToolNamesFromSet(values, LIGHTWEIGHT_HEADLESS_TOOL_NAMES)
}

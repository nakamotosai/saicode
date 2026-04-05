const FALLBACK_VERSION = '1.0.0'

export function getSaicodeCliVersion(): string {
  if (
    typeof MACRO !== 'undefined' &&
    typeof MACRO.VERSION === 'string' &&
    MACRO.VERSION.trim()
  ) {
    return MACRO.VERSION
  }

  return FALLBACK_VERSION
}

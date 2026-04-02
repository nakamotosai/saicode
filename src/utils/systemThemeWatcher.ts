import {
  getSystemThemeName,
  setCachedSystemTheme,
  themeFromOscColor,
  type SystemTheme,
} from './systemTheme.js'

export function watchSystemTheme(
  _internalQuerier: unknown,
  onTheme: (theme: SystemTheme) => void,
): () => void {
  const initial = getSystemThemeName()
  setCachedSystemTheme(initial)
  onTheme(initial)

  return () => {}
}

export function applyOscThemeSample(sample: string): SystemTheme | undefined {
  const nextTheme = themeFromOscColor(sample)
  if (nextTheme) {
    setCachedSystemTheme(nextTheme)
  }
  return nextTheme
}


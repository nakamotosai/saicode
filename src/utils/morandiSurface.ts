const MORANDI_SURFACE_COLORS = [
  'rainbow_yellow',
  'rainbow_green',
  'rainbow_blue',
  'rainbow_violet',
  'rainbow_red',
] as const

export type MorandiSurfaceColor = (typeof MORANDI_SURFACE_COLORS)[number]

function hashString(text: string): number {
  let hash = 0
  for (let index = 0; index < text.length; index++) {
    hash = (hash * 31 + text.charCodeAt(index)) >>> 0
  }
  return hash
}

export function getMorandiSurfaceColorFromText(text: string): MorandiSurfaceColor {
  if (!text) {
    return MORANDI_SURFACE_COLORS[0]
  }
  return MORANDI_SURFACE_COLORS[hashString(text) % MORANDI_SURFACE_COLORS.length]!
}

export function getMorandiSurfaceColorFromNumber(seed: number): MorandiSurfaceColor {
  return MORANDI_SURFACE_COLORS[Math.abs(seed) % MORANDI_SURFACE_COLORS.length]!
}

export function getMorandiSurfaceColorFromIndex(index: number): MorandiSurfaceColor {
  return MORANDI_SURFACE_COLORS[Math.abs(index) % MORANDI_SURFACE_COLORS.length]!
}

function stripCodeFence(raw: string): string {
  const trimmed = raw.trim()
  const fenced = trimmed.match(/^```(?:json)?\s*([\s\S]*?)\s*```$/i)
  return fenced?.[1]?.trim() ?? trimmed
}

function tryParseJsonRecursively(raw: string): unknown | null {
  let current = stripCodeFence(raw)
  for (let i = 0; i < 2; i += 1) {
    try {
      const parsed = JSON.parse(current)
      if (typeof parsed === 'string') {
        current = parsed
        continue
      }
      return parsed
    } catch {
      return null
    }
  }
  return null
}

function extractConcatenatedJsonObjects(raw: string): string[] {
  const text = stripCodeFence(raw)
  const results: string[] = []
  let depth = 0
  let start = -1
  let inString = false
  let escaped = false

  for (let i = 0; i < text.length; i += 1) {
    const ch = text[i]

    if (inString) {
      if (escaped) {
        escaped = false
        continue
      }
      if (ch === '\\') {
        escaped = true
        continue
      }
      if (ch === '"') {
        inString = false
      }
      continue
    }

    if (ch === '"') {
      inString = true
      continue
    }

    if (ch === '{') {
      if (depth === 0) {
        start = i
      }
      depth += 1
      continue
    }

    if (ch === '}') {
      if (depth === 0) {
        continue
      }
      depth -= 1
      if (depth === 0 && start !== -1) {
        results.push(text.slice(start, i + 1))
        start = -1
      }
    }
  }

  return results
}

function mergeParsedObjects(values: unknown[]): Record<string, unknown> | null {
  const merged: Record<string, unknown> = {}
  let foundObject = false

  for (const value of values) {
    if (!value || typeof value !== 'object' || Array.isArray(value)) {
      continue
    }
    foundObject = true
    for (const [key, candidate] of Object.entries(value)) {
      if (!(key in merged)) {
        merged[key] = candidate
      }
    }
  }

  return foundObject ? merged : null
}

export function repairMalformedToolArguments(raw: string): unknown {
  const direct = tryParseJsonRecursively(raw)
  if (direct !== null) {
    return direct
  }

  const objects = extractConcatenatedJsonObjects(raw)
  if (objects.length > 0) {
    const parsed = objects
      .map(chunk => tryParseJsonRecursively(chunk))
      .filter(value => value !== null)
    const merged = mergeParsedObjects(parsed)
    if (merged) {
      return merged
    }
  }

  return { raw: stripCodeFence(raw) }
}

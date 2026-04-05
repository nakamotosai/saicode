import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process'
import { existsSync, unlinkSync } from 'node:fs'
import { createServer, type Socket } from 'node:net'
import { fileURLToPath } from 'node:url'
import {
  applyLightweightHeadlessProcessPrelude,
  initializeLightweightHeadlessRuntime,
  resolveLightweightHeadlessExecutionRequest,
  runLightweightHeadlessExecutionRequest,
  shouldFallbackToFullCli,
  type LightweightHeadlessExecutionRequest,
} from './headlessPrint.js'

const READY_SENTINEL = '__SAICODE_WARM_READY__'

type WarmChildMode = 'lightweight' | 'simple'

type WarmManagerRequest = {
  argv: string[]
  cwd: string
  envFingerprint: string
}

type WarmManagerResponse = {
  ok: boolean
  exitCode?: number
  stdout?: string
  stderr?: string
  fallbackReason?: string
  restartRequired?: boolean
}

type WarmChildPayload = {
  request: LightweightHeadlessExecutionRequest
}

type WarmChildKey = {
  mode: WarmChildMode
  cwd: string
}

type WarmChildHandle = {
  child: ChildProcessWithoutNullStreams
  key: WarmChildKey
  stdoutChunks: Buffer[]
  stderrChunks: Buffer[]
}

function warmChildModeForRequest(
  request: LightweightHeadlessExecutionRequest,
): WarmChildMode {
  return request.shouldEnableSimpleMode ? 'simple' : 'lightweight'
}

function sameChildKey(left: WarmChildKey, right: WarmChildKey): boolean {
  return left.mode === right.mode && left.cwd === right.cwd
}

async function readText(stream: NodeJS.ReadableStream): Promise<string> {
  const chunks: Buffer[] = []
  for await (const chunk of stream) {
    chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(String(chunk)))
  }
  return Buffer.concat(chunks).toString('utf8')
}

async function readFrame(stream: NodeJS.ReadableStream): Promise<string> {
  return await new Promise<string>((resolve, reject) => {
    let buffer = ''

    const cleanup = () => {
      stream.off('data', onData)
      stream.off('end', onEnd)
      stream.off('error', onError)
    }

    const onData = (chunk: Buffer | string) => {
      buffer += Buffer.isBuffer(chunk) ? chunk.toString('utf8') : String(chunk)
      const newlineIndex = buffer.indexOf('\n')
      if (newlineIndex === -1) {
        return
      }

      cleanup()
      resolve(buffer.slice(0, newlineIndex))
    }

    const onEnd = () => {
      cleanup()
      resolve(buffer)
    }

    const onError = (error: Error) => {
      cleanup()
      reject(error)
    }

    stream.on('data', onData)
    stream.once('end', onEnd)
    stream.once('error', onError)
  })
}

function safeUnlink(path: string): void {
  if (!existsSync(path)) {
    return
  }

  try {
    unlinkSync(path)
  } catch {}
}

function killChild(child: ChildProcessWithoutNullStreams): void {
  if (child.killed) {
    return
  }

  try {
    child.kill('SIGTERM')
  } catch {}
}

async function spawnWarmChild(key: WarmChildKey): Promise<WarmChildHandle> {
  const scriptPath = fileURLToPath(import.meta.url)
  const stdoutChunks: Buffer[] = []
  const stderrChunks: Buffer[] = []

  return await new Promise<WarmChildHandle>((resolve, reject) => {
    const child = spawn(process.execPath, [scriptPath, '--child', key.mode], {
      cwd: key.cwd,
      env: {
        ...process.env,
        SAICODE_WARM_CHILD_MODE: key.mode,
      },
      stdio: ['pipe', 'pipe', 'pipe'],
    })

    let ready = false
    let stderrPreReady = ''

    const fail = (error: Error) => {
      killChild(child)
      reject(error)
    }

    child.stdout.on('data', chunk => {
      stdoutChunks.push(Buffer.from(chunk))
    })

    child.stderr.on('data', chunk => {
      if (ready) {
        stderrChunks.push(Buffer.from(chunk))
        return
      }

      stderrPreReady += Buffer.from(chunk).toString('utf8')
      const marker = `${READY_SENTINEL}\n`
      const markerIndex = stderrPreReady.indexOf(marker)
      if (markerIndex === -1) {
        if (stderrPreReady.length > 64 * 1024) {
          fail(
            new Error(
              `Warm child exceeded stderr prelude budget before ready (${key.mode})`,
            ),
          )
        }
        return
      }

      ready = true
      const before = stderrPreReady.slice(0, markerIndex)
      const after = stderrPreReady.slice(markerIndex + marker.length)
      if (before.length > 0) {
        stderrChunks.push(Buffer.from(before))
      }
      if (after.length > 0) {
        stderrChunks.push(Buffer.from(after))
      }

      resolve({
        child,
        key,
        stdoutChunks,
        stderrChunks,
      })
    })

    child.once('error', error => {
      if (!ready) {
        reject(error)
      }
    })

    child.once('exit', (code, signal) => {
      if (ready) {
        return
      }

      const detail = stderrPreReady.trim()
      reject(
        new Error(
          [
            `Warm child exited before ready (mode=${key.mode}, code=${code ?? 'null'}, signal=${signal ?? 'null'})`,
            detail,
          ]
            .filter(Boolean)
            .join('\n'),
        ),
      )
    })
  })
}

async function runWarmChild(
  handle: WarmChildHandle,
  request: LightweightHeadlessExecutionRequest,
): Promise<WarmManagerResponse> {
  const payload: WarmChildPayload = { request }
  handle.child.stdin.end(JSON.stringify(payload))

  const exitCode = await new Promise<number>(resolve => {
    handle.child.once('exit', code => {
      resolve(code ?? 1)
    })
  })

  return {
    ok: true,
    exitCode,
    stdout: Buffer.concat(handle.stdoutChunks).toString('utf8'),
    stderr: Buffer.concat(handle.stderrChunks).toString('utf8'),
  }
}

async function respond(socket: Socket, response: WarmManagerResponse): Promise<void> {
  await new Promise<void>((resolve, reject) => {
    socket.once('error', reject)
    socket.end(JSON.stringify(response), () => {
      socket.off('error', reject)
      resolve()
    })
  })
}

async function runManager(socketPath: string): Promise<void> {
  const expectedEnvFingerprint =
    process.env.SAICODE_WARM_ENV_FINGERPRINT ?? ''

  let requestInFlight = false
  let readyChildKey: WarmChildKey | null = null
  let readyChildPromise: Promise<WarmChildHandle> | null = null
  let shutdownRequested = false

  const forgetReadyChild = () => {
    if (!readyChildPromise) {
      readyChildKey = null
      return
    }

    const stalePromise = readyChildPromise
    readyChildPromise = null
    readyChildKey = null
    void stalePromise.then(handle => {
      killChild(handle.child)
    }).catch(() => {})
  }

  const schedulePrewarm = (key: WarmChildKey) => {
    if (
      readyChildPromise &&
      readyChildKey &&
      sameChildKey(readyChildKey, key)
    ) {
      return
    }

    forgetReadyChild()
    readyChildKey = key
    const promise = spawnWarmChild(key)
    readyChildPromise = promise
    void promise.catch(() => {
      if (readyChildPromise === promise) {
        readyChildPromise = null
        readyChildKey = null
      }
    })
  }

  const claimChild = async (key: WarmChildKey): Promise<WarmChildHandle> => {
    if (
      readyChildPromise &&
      readyChildKey &&
      sameChildKey(readyChildKey, key)
    ) {
      const promise = readyChildPromise
      readyChildPromise = null
      readyChildKey = null
      return await promise
    }

    forgetReadyChild()
    return await spawnWarmChild(key)
  }

  safeUnlink(socketPath)

  const server = createServer(socket => {
    void (async () => {
      let response: WarmManagerResponse

      try {
        const raw = await readFrame(socket)
        const request = JSON.parse(raw) as WarmManagerRequest

        if (request.envFingerprint !== expectedEnvFingerprint) {
          shutdownRequested = true
          response = {
            ok: false,
            fallbackReason: 'env_mismatch',
            restartRequired: true,
          }
          await respond(socket, response)
          server.close()
          return
        }

        if (requestInFlight) {
          response = {
            ok: false,
            fallbackReason: 'warm_worker_busy',
          }
          await respond(socket, response)
          return
        }

        requestInFlight = true

        const executionRequest =
          await resolveLightweightHeadlessExecutionRequest(request.argv, {
            allowStdin: false,
          })

        if (!executionRequest.prompt.trim()) {
          response = {
            ok: false,
            fallbackReason: 'missing_prompt_argv',
          }
          await respond(socket, response)
          return
        }

        if (shouldFallbackToFullCli(executionRequest)) {
          response = {
            ok: false,
            fallbackReason: 'slash_prompt_requires_full_cli',
          }
          await respond(socket, response)
          return
        }

        const key: WarmChildKey = {
          mode: warmChildModeForRequest(executionRequest),
          cwd: request.cwd,
        }
        const child = await claimChild(key)
        response = await runWarmChild(child, executionRequest)
        await respond(socket, response)
        schedulePrewarm(key)
      } catch (error) {
        response = {
          ok: false,
          fallbackReason:
            error instanceof Error ? error.message : String(error),
        }
        try {
          await respond(socket, response)
        } catch {}
      } finally {
        requestInFlight = false
      }
    })()
  })

  const closeServer = () => {
    shutdownRequested = true
    forgetReadyChild()
    server.close()
  }

  process.on('SIGINT', closeServer)
  process.on('SIGTERM', closeServer)
  process.on('exit', () => {
    forgetReadyChild()
    safeUnlink(socketPath)
  })

  await new Promise<void>((resolve, reject) => {
    server.once('error', reject)
    server.listen(socketPath, () => {
      server.off('error', reject)
      resolve()
    })
  })

  schedulePrewarm({
    mode: 'lightweight',
    cwd: process.cwd(),
  })

  await new Promise<void>((resolve, reject) => {
    server.once('close', resolve)
    server.once('error', reject)
  })

  if (!shutdownRequested) {
    safeUnlink(socketPath)
  }
}

async function runChild(mode: WarmChildMode): Promise<void> {
  applyLightweightHeadlessProcessPrelude(mode === 'simple')
  await initializeLightweightHeadlessRuntime()
  process.stderr.write(`${READY_SENTINEL}\n`)

  const raw = await readText(process.stdin)
  if (!raw.trim()) {
    throw new Error('Warm child missing request payload')
  }

  const payload = JSON.parse(raw) as WarmChildPayload
  if (!payload.request) {
    throw new Error('Warm child payload missing request')
  }

  const expectedMode = warmChildModeForRequest(payload.request)
  if (expectedMode !== mode) {
    throw new Error(
      `Warm child mode mismatch: expected ${expectedMode}, got ${mode}`,
    )
  }

  await runLightweightHeadlessExecutionRequest(payload.request)
}

function getArgValue(flag: '--manager' | '--child'): string | undefined {
  const index = process.argv.indexOf(flag)
  if (index === -1) {
    return undefined
  }
  return process.argv[index + 1]
}

export async function main(): Promise<void> {
  const socketPath = getArgValue('--manager')
  if (socketPath) {
    await runManager(socketPath)
    return
  }

  const mode = getArgValue('--child')
  if (mode === 'simple' || mode === 'lightweight') {
    await runChild(mode)
    return
  }

  throw new Error(
    'headlessPrintWarmWorker requires either --manager <socket-path> or --child <mode>',
  )
}

if (import.meta.main) {
  void main().catch(error => {
    const message =
      error instanceof Error ? error.stack || error.message : String(error)
    process.stderr.write(`${message}\n`)
    process.exitCode = 1
  })
}

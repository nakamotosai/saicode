import { useMemo } from 'react'

type UseSSHSessionResult = {
  isRemoteMode: boolean
  sendMessage: (_content?: unknown, _opts?: { uuid?: string }) => Promise<boolean>
  cancelRequest: () => void
  disconnect: () => void
}

type UseSSHSessionProps = {
  session: unknown
  setMessages?: unknown
  setIsLoading?: unknown
  setToolUseConfirmQueue?: unknown
  tools?: unknown
}

export function useSSHSession(
  _props: UseSSHSessionProps,
): UseSSHSessionResult {
  return useMemo(
    () => ({
      isRemoteMode: false,
      sendMessage: async () => false,
      cancelRequest: () => {},
      disconnect: () => {},
    }),
    [],
  )
}

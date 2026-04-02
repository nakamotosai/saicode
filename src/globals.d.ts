declare const MACRO: Record<string, unknown>

declare namespace React {
  type ReactNode = any
  type SetStateAction<S> = S | ((prevState: S) => S)
  type Dispatch<A> = (value: A) => void
}

declare const Gates: any
declare const apiMetricsRef: any
declare const computeTtftText: any
declare const TungstenPill: any
declare const getAntModelOverrideConfig: any

declare namespace NodeJS {
  interface ProcessEnv {
    USER_TYPE?: string
    NODE_ENV?: string
    APP_ENV?: string
    IS_DEMO?: string
    DEMO_VERSION?: string
    CLAUDE_CODE_ENTRYPOINT?: string
    CLAUDE_CODE_STREAMLINED_OUTPUT?: string
    CLAUDE_CODE_TMUX_SESSION?: string
    CLAUDE_CODE_TMUX_PREFIX?: string
    CLAUDE_CODE_TMUX_PREFIX_CONFLICTS?: string
    CLAUDE_CODE_SESSION_ID?: string
    CLAUDE_CODE_SESSION_ACCESS_TOKEN?: string
  }
}

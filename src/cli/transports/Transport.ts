export interface Transport {
  connect?(): void | Promise<void>
  close(): void | Promise<void>
  send?(message: unknown): void | Promise<void>
  write?(message: unknown): void | Promise<void>
  setOnData?(handler: (data: unknown) => void): void
  setOnClose?(handler: () => void): void
}

export default Transport

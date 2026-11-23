export interface Senryu {
  message_id: string
  channel_id: string
  verse: string
  detected_at: string
  source_url: string
}

export interface SenryuPage {
  items: Senryu[]
  next_cursor: string | null
}

export interface CommandCatalog {
  mention: string
  commands: { usage: string; description: string }[]
}

async function get<T>(path: string, signal: AbortSignal): Promise<T> {
  const response = await fetch('/api' + path, {
    signal,
    headers: { Accept: 'application/json' }
  })
  if (response.status === 401 || response.status === 403) {
    throw new Error('閲覧には部員認証が必要です。')
  }
  if (!response.ok) {
    throw new Error('データを取得できませんでした。')
  }
  if (!response.headers.get('content-type')?.includes('application/json')) {
    throw new Error('API の応答を読み込めませんでした。')
  }
  return (await response.json()) as T
}

export const api = {
  senryu: (before: string | null, signal: AbortSignal) =>
    get<SenryuPage>(
      before ? '/senryu?before=' + encodeURIComponent(before) : '/senryu',
      signal
    ),
  commands: (signal: AbortSignal) => get<CommandCatalog>('/commands', signal)
}

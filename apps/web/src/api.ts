import { useAuth, type Session } from '@/store'
export interface Item {
  id: string
  title: string
  description: string
  completed: boolean
  created_at: string
  updated_at: string
}
export type ItemInput = Pick<Item, 'title' | 'description' | 'completed'>
export class ApiError extends Error {
  constructor(
    public status: number,
    message: string
  ) {
    super(message)
  }
}
async function request<T>(path: string, options: RequestInit = {}): Promise<T> {
  const token = useAuth.getState().session?.token
  const response = await fetch('/api' + path, {
    ...options,
    headers: {
      ...(options.body ? { 'Content-Type': 'application/json' } : {}),
      ...(token ? { Authorization: 'Bearer ' + token } : {}),
      ...options.headers
    }
  })
  if (!response.ok) {
    const body = (await response.json().catch(() => null)) as {
      message?: string
    } | null
    if (response.status === 401) useAuth.getState().setSession(null)
    throw new ApiError(
      response.status,
      body?.message ?? 'リクエストに失敗しました (' + response.status + ')'
    )
  }
  return response.status === 204
    ? (undefined as T)
    : (response.json() as Promise<T>)
}
export const api = {
  health: () => request<{ status: string }>('/health'),
  auth: (mode: 'login' | 'register', email: string, password: string) =>
    request<Session>('/auth/' + mode, {
      method: 'POST',
      body: JSON.stringify({ email, password })
    }),
  items: () => request<Item[]>('/items'),
  item: (id: string) => request<Item>('/items/' + encodeURIComponent(id)),
  create: (input: ItemInput) =>
    request<Item>('/items', { method: 'POST', body: JSON.stringify(input) }),
  update: (id: string, input: ItemInput) =>
    request<Item>('/items/' + encodeURIComponent(id), {
      method: 'PUT',
      body: JSON.stringify(input)
    }),
  delete: (id: string) =>
    request<void>('/items/' + encodeURIComponent(id), { method: 'DELETE' })
}

export const errorMessage = (error: unknown) =>
  error instanceof Error ? error.message : 'エラーが発生しました'

import { create } from 'zustand'
import { persist } from 'zustand/middleware'
export interface User {
  id: string
  email: string
  created_at: string
}
export interface Session {
  token: string
  user: User
}
// Tokens stay in memory. Reloading the page requires signing in again.
export const useAuth = create<{
  session: Session | null
  setSession: (session: Session | null) => void
}>(set => ({
  session: null,
  setSession: session => set({ session })
}))
export const usePreferences = create(
  persist<{ mode: 'light' | 'dark'; toggle: () => void }>(
    set => ({
      mode: 'light',
      toggle: () => set(s => ({ mode: s.mode === 'light' ? 'dark' : 'light' }))
    }),
    { name: 'starter-preferences' }
  )
)

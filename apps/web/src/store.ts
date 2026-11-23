import { create } from 'zustand'
import { persist } from 'zustand/middleware'

export const usePreferences = create(
  persist<{ mode: 'light' | 'dark'; toggle: () => void }>(
    set => ({
      mode: 'light',
      toggle: () =>
        set(state => ({ mode: state.mode === 'light' ? 'dark' : 'light' }))
    }),
    { name: 'rachite-preferences' }
  )
)

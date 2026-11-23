import React, { useMemo } from 'react'
import ReactDOM from 'react-dom/client'
import { BrowserRouter } from 'react-router-dom'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { CssBaseline, ThemeProvider } from '@mui/material'

import App from '@/App'
import { usePreferences } from '@/store'
import { createRachiteTheme } from '@/theme'
import './style.css'

const client = new QueryClient({
  defaultOptions: {
    queries: { staleTime: 30_000, retry: false }
  }
})

function Root() {
  const mode = usePreferences(state => state.mode)
  const theme = useMemo(() => createRachiteTheme(mode), [mode])

  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <QueryClientProvider client={client}>
        <BrowserRouter>
          <App />
        </BrowserRouter>
      </QueryClientProvider>
    </ThemeProvider>
  )
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <Root />
  </React.StrictMode>
)

import React, { useMemo } from 'react'
import ReactDOM from 'react-dom/client'
import { BrowserRouter } from 'react-router-dom'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { CssBaseline, ThemeProvider, createTheme } from '@mui/material'
import { SnackbarProvider } from 'notistack'
import { usePreferences } from '@/store'
import App from '@/App'
import './style.css'
const client = new QueryClient({
  defaultOptions: {
    queries: { staleTime: 30_000, retry: false },
    mutations: { retry: false }
  }
})
function Root() {
  const mode = usePreferences(s => s.mode)
  const theme = useMemo(
    () =>
      createTheme({
        palette: {
          mode,
          primary: { main: mode === 'light' ? '#245b49' : '#8bd4b5' },
          background: { default: mode === 'light' ? '#f5f7f8' : '#101916' }
        },
        typography: {
          fontFamily: '"Noto Sans JP", system-ui, sans-serif',
          h3: { fontWeight: 750 },
          h4: { fontWeight: 700 },
          button: { textTransform: 'none', fontWeight: 650 }
        },
        shape: { borderRadius: 12 },
        components: {
          MuiButton: { defaultProps: { disableElevation: true } },
          MuiCard: { defaultProps: { variant: 'outlined' } }
        }
      }),
    [mode]
  )
  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <QueryClientProvider client={client}>
        <SnackbarProvider maxSnack={3} autoHideDuration={3500}>
          <BrowserRouter>
            <App />
          </BrowserRouter>
        </SnackbarProvider>
      </QueryClientProvider>
    </ThemeProvider>
  )
}
ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <Root />
  </React.StrictMode>
)

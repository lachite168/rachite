import { lazy, Suspense, useEffect } from 'react'
import { Link as RouterLink, Route, Routes } from 'react-router-dom'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import {
  Alert,
  AppBar,
  Button,
  CircularProgress,
  Container,
  Divider,
  IconButton,
  Link,
  Stack,
  Toolbar,
  Tooltip,
  Typography
} from '@mui/material'
import {
  DarkModeOutlined,
  LightModeOutlined,
  WidgetsOutlined
} from '@mui/icons-material'
import { api } from '@/api'
import { useAuth, usePreferences } from '@/store'
const Auth = lazy(() => import('@/pages/Auth'))
const Items = lazy(() => import('@/pages/Items'))
const Detail = lazy(() => import('@/pages/Detail'))
const About = lazy(() => import('@/pages/About'))
export default function App() {
  const session = useAuth(s => s.session)
  const setSession = useAuth(s => s.setSession)
  const { mode, toggle } = usePreferences()
  const client = useQueryClient()
  useEffect(
    () =>
      useAuth.subscribe((state, previous) => {
        if (state.session?.user.id !== previous.session?.user.id) client.clear()
      }),
    [client]
  )
  const health = useQuery({
    queryKey: ['health'],
    queryFn: api.health,
    refetchInterval: 60_000
  })
  return (
    <>
      <AppBar
        position="static"
        color="inherit"
        elevation={0}
        sx={{ borderBottom: 1, borderColor: 'divider' }}
      >
        <Container maxWidth="lg">
          <Toolbar
            disableGutters
            sx={{ gap: { xs: 1, sm: 3 }, flexWrap: 'wrap', py: 1 }}
          >
            <Stack
              direction="row"
              spacing={1}
              sx={{ alignItems: 'center', flexGrow: 1 }}
            >
              <WidgetsOutlined color="primary" />
              <Link
                component={RouterLink}
                to="/"
                color="text.primary"
                underline="none"
                sx={{ fontWeight: 800 }}
              >
                Fullstack Starter
              </Link>
            </Stack>
            <Button component={RouterLink} to="/">
              アイテム
            </Button>
            <Button component={RouterLink} to="/about">
              About
            </Button>
            <Tooltip title={mode === 'light' ? 'ダークモード' : 'ライトモード'}>
              <IconButton onClick={toggle} aria-label="テーマを切り替え">
                {mode === 'light' ? (
                  <DarkModeOutlined />
                ) : (
                  <LightModeOutlined />
                )}
              </IconButton>
            </Tooltip>
            {session && (
              <Button
                onClick={() => {
                  setSession(null)
                  client.clear()
                }}
              >
                ログアウト
              </Button>
            )}
          </Toolbar>
        </Container>
      </AppBar>
      <Container
        component="main"
        maxWidth="lg"
        sx={{ py: { xs: 4, sm: 6 }, minHeight: 'calc(100vh - 180px)' }}
      >
        <Suspense fallback={<CircularProgress aria-label="読み込み中" />}>
          <Routes>
            <Route path="/" element={session ? <Items /> : <Auth />} />
            <Route
              path="/items/:id"
              element={session ? <Detail /> : <Auth />}
            />
            <Route path="/about" element={<About />} />
            <Route
              path="*"
              element={
                <Alert severity="info">
                  ページが見つかりません。
                  <Link component={RouterLink} to="/">
                    ホームへ戻る
                  </Link>
                </Alert>
              }
            />
          </Routes>
        </Suspense>
      </Container>
      <Container maxWidth="lg">
        <Divider />
        <Stack
          direction={{ xs: 'column', sm: 'row' }}
          spacing={1}
          sx={{ justifyContent: 'space-between', py: 3 }}
        >
          <Typography variant="body2" color="text.secondary">
            Rust × React · Fullstack Starter
          </Typography>
          <Typography variant="body2" color="text.secondary">
            {session?.user.email && session.user.email + ' · '}
            {health.isSuccess
              ? 'API 接続済み'
              : health.isError
                ? 'API に接続できません'
                : 'API 接続確認中'}
          </Typography>
        </Stack>
      </Container>
    </>
  )
}

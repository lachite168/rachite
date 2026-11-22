import { useState, type FormEvent } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import {
  Alert,
  Button,
  Card,
  CardContent,
  Stack,
  TextField,
  Typography
} from '@mui/material'
import { api, errorMessage as message } from '@/api'
import { useAuth } from '@/store'
export default function Auth() {
  const [mode, setMode] = useState<'login' | 'register'>('login')
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const setSession = useAuth(s => s.setSession)
  const client = useQueryClient()
  const mutation = useMutation({
    mutationFn: () => api.auth(mode, email, password),
    onSuccess: session => {
      client.clear()
      setSession(session)
    }
  })
  function submit(event: FormEvent) {
    event.preventDefault()
    mutation.mutate()
  }
  return (
    <Card sx={{ maxWidth: 440, mx: 'auto', my: 6 }}>
      <CardContent sx={{ p: 4 }}>
        <Typography variant="h5" sx={{ fontWeight: 700, mb: 1 }}>
          {mode === 'login' ? 'おかえりなさい' : 'アカウントを作成'}
        </Typography>
        <Typography color="text.secondary" sx={{ mb: 3 }}>
          自分のアイテムを管理して、開発をはじめましょう。
        </Typography>
        <Stack component="form" onSubmit={submit} spacing={2}>
          {mutation.isError && (
            <Alert severity="error">{message(mutation.error)}</Alert>
          )}
          <TextField
            label="メールアドレス"
            type="email"
            autoComplete="email"
            value={email}
            onChange={e => setEmail(e.target.value)}
            required
            slotProps={{ htmlInput: { maxLength: 254 } }}
          />
          <TextField
            label="パスワード"
            type="password"
            autoComplete={
              mode === 'login' ? 'current-password' : 'new-password'
            }
            helperText="12〜128バイト（英数字なら12〜128文字）"
            value={password}
            onChange={e => setPassword(e.target.value)}
            required
            slotProps={{ htmlInput: { minLength: 12, maxLength: 128 } }}
          />
          <Button
            type="submit"
            variant="contained"
            size="large"
            disabled={mutation.isPending}
          >
            {mutation.isPending
              ? '処理中…'
              : mode === 'login'
                ? 'ログイン'
                : '登録してはじめる'}
          </Button>
          <Button
            onClick={() => {
              setMode(mode === 'login' ? 'register' : 'login')
              mutation.reset()
            }}
          >
            {mode === 'login' ? 'アカウントを新規作成' : 'ログインに戻る'}
          </Button>
        </Stack>
      </CardContent>
    </Card>
  )
}

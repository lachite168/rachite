import { useState } from 'react'
import { Link as RouterLink } from 'react-router-dom'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import {
  Alert,
  Box,
  Button,
  Card,
  CardContent,
  Checkbox,
  Chip,
  CircularProgress,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Link,
  Stack,
  Typography
} from '@mui/material'
import { Add, CheckCircleOutlined } from '@mui/icons-material'
import { useSnackbar } from 'notistack'
import { api, errorMessage as message, type Item } from '@/api'
import { useAuth } from '@/store'
import dayjs from '@/lib/date'
import ItemEditor from '@/components/ItemEditor'
export default function Items() {
  const [open, setOpen] = useState(false)
  const [filter, setFilter] = useState<'all' | 'active' | 'done'>('all')
  const client = useQueryClient()
  const userId = useAuth(s => s.session?.user.id)
  const { enqueueSnackbar } = useSnackbar()
  const query = useQuery({ queryKey: ['items', userId], queryFn: api.items })
  const create = useMutation({
    mutationFn: api.create,
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ['items'] })
      setOpen(false)
      enqueueSnackbar('アイテムを追加しました', { variant: 'success' })
    },
    onError: error => enqueueSnackbar(message(error), { variant: 'error' })
  })
  const toggle = useMutation({
    mutationFn: (item: Item) =>
      api.update(item.id, { ...item, completed: !item.completed }),
    onSuccess: item => {
      client.setQueryData(['item', userId, item.id], item)
      void client.invalidateQueries({ queryKey: ['items'] })
    },
    onError: error => enqueueSnackbar(message(error), { variant: 'error' })
  })
  const items = query.data ?? []
  const visible = items.filter(
    i => filter === 'all' || (filter === 'done' ? i.completed : !i.completed)
  )
  return (
    <>
      <Stack
        direction={{ xs: 'column', sm: 'row' }}
        spacing={2}
        sx={{
          justifyContent: 'space-between',
          alignItems: { sm: 'center' },
          mb: 4
        }}
      >
        <Box>
          <Typography variant="overline" color="primary">
            YOUR WORKSPACE
          </Typography>
          <Typography
            variant="h3"
            sx={{ fontSize: { xs: '2rem', sm: '2.75rem' } }}
          >
            アイテム
          </Typography>
          <Typography color="text.secondary" sx={{ mt: 1 }}>
            小さなアイデアから、次のプロダクトへ。
          </Typography>
        </Box>
        <Button
          variant="contained"
          startIcon={<Add />}
          onClick={() => setOpen(true)}
        >
          アイテムを追加
        </Button>
      </Stack>
      <Stack direction="row" sx={{ gap: 1, mb: 3, flexWrap: 'wrap' }}>
        {(
          [
            ['all', 'すべて'],
            ['active', '未完了'],
            ['done', '完了']
          ] as const
        ).map(([value, label]) => (
          <Chip
            key={value}
            label={
              label +
              ' · ' +
              items.filter(
                i =>
                  value === 'all' ||
                  (value === 'done' ? i.completed : !i.completed)
              ).length
            }
            color={filter === value ? 'primary' : 'default'}
            onClick={() => setFilter(value)}
            variant={filter === value ? 'filled' : 'outlined'}
          />
        ))}
      </Stack>
      {query.isPending ? (
        <CircularProgress aria-label="読み込み中" />
      ) : query.isError ? (
        <Alert
          severity="error"
          action={<Button onClick={() => void query.refetch()}>再試行</Button>}
        >
          {message(query.error)}
        </Alert>
      ) : (
        <Stack spacing={1.5}>
          {visible.map(item => (
            <Card key={item.id}>
              <Stack
                direction="row"
                spacing={2}
                sx={{ alignItems: 'center', p: 2 }}
              >
                <Checkbox
                  checked={item.completed}
                  disabled={toggle.isPending}
                  onChange={() => toggle.mutate(item)}
                  slotProps={{
                    input: {
                      'aria-label':
                        item.title +
                        'を' +
                        (item.completed ? '未完了' : '完了') +
                        'にする'
                    }
                  }}
                />
                <Box sx={{ minWidth: 0, flex: 1 }}>
                  <Link
                    component={RouterLink}
                    to={'/items/' + item.id}
                    underline="hover"
                    color="text.primary"
                    sx={{
                      fontWeight: 650,
                      overflowWrap: 'anywhere',
                      textDecoration: item.completed
                        ? 'line-through'
                        : undefined
                    }}
                  >
                    {item.title}
                  </Link>
                  <Typography color="text.secondary" noWrap sx={{ mt: 0.5 }}>
                    {item.description || '説明はありません'}
                  </Typography>
                </Box>
                <Typography
                  variant="body2"
                  color="text.secondary"
                  sx={{ display: { xs: 'none', sm: 'block' } }}
                >
                  {dayjs(item.updated_at).fromNow()}
                </Typography>
              </Stack>
            </Card>
          ))}
          {visible.length === 0 && (
            <Card>
              <CardContent sx={{ textAlign: 'center', py: 7 }}>
                <CheckCircleOutlined
                  color="primary"
                  sx={{ fontSize: 42, mb: 2 }}
                />
                <Typography variant="h6">ここから始めましょう</Typography>
                <Typography color="text.secondary" sx={{ mt: 1 }}>
                  {items.length
                    ? 'この条件のアイテムはありません。'
                    : '「アイテムを追加」から最初のアイデアを登録できます。'}
                </Typography>
              </CardContent>
            </Card>
          )}
        </Stack>
      )}
      <Dialog
        open={open}
        onClose={() => !create.isPending && setOpen(false)}
        fullWidth
        maxWidth="sm"
      >
        <DialogTitle>新しいアイテム</DialogTitle>
        <DialogContent>
          <ItemEditor
            onSave={input => create.mutate(input)}
            pending={create.isPending}
          />
        </DialogContent>
        <DialogActions>
          <Button disabled={create.isPending} onClick={() => setOpen(false)}>
            キャンセル
          </Button>
        </DialogActions>
      </Dialog>
    </>
  )
}

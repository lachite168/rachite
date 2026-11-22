import { useState } from 'react'
import { Link as RouterLink, useNavigate, useParams } from 'react-router-dom'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import {
  Alert,
  Box,
  Button,
  Card,
  CardContent,
  CircularProgress,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Divider,
  Typography
} from '@mui/material'
import { DeleteOutlined } from '@mui/icons-material'
import { useSnackbar } from 'notistack'
import { api, errorMessage as message, type ItemInput } from '@/api'
import { useAuth } from '@/store'
import dayjs from '@/lib/date'
import ItemEditor from '@/components/ItemEditor'
export default function Detail() {
  const { id = '' } = useParams()
  const userId = useAuth(s => s.session?.user.id)
  const client = useQueryClient()
  const navigate = useNavigate()
  const { enqueueSnackbar } = useSnackbar()
  const [confirm, setConfirm] = useState(false)
  const query = useQuery({
    queryKey: ['item', userId, id],
    queryFn: () => api.item(id)
  })
  const update = useMutation({
    mutationFn: (input: ItemInput) => api.update(id, input),
    onSuccess: item => {
      client.setQueryData(['item', userId, id], item)
      void client.invalidateQueries({ queryKey: ['items'] })
      enqueueSnackbar('保存しました', { variant: 'success' })
    },
    onError: error => enqueueSnackbar(message(error), { variant: 'error' })
  })
  const remove = useMutation({
    mutationFn: () => api.delete(id),
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ['items'] })
      client.removeQueries({ queryKey: ['item', userId, id] })
      navigate('/')
      enqueueSnackbar('削除しました', { variant: 'success' })
    },
    onError: error => enqueueSnackbar(message(error), { variant: 'error' })
  })
  return (
    <Box sx={{ maxWidth: 700, mx: 'auto' }}>
      <Button component={RouterLink} to="/" sx={{ mb: 2 }}>
        一覧に戻る
      </Button>
      {query.isPending ? (
        <CircularProgress aria-label="読み込み中" />
      ) : query.isError ? (
        <Alert severity="error">{message(query.error)}</Alert>
      ) : (
        <Card>
          <CardContent sx={{ p: { xs: 2, sm: 4 } }}>
            <Typography variant="h4" sx={{ mb: 1 }}>
              アイテムを編集
            </Typography>
            <Typography color="text.secondary" sx={{ mb: 3 }}>
              作成日 {dayjs(query.data.created_at).format('YYYY年M月D日 HH:mm')}
            </Typography>
            <ItemEditor
              key={id}
              initial={query.data}
              onSave={input => update.mutate(input)}
              pending={update.isPending || remove.isPending}
            />
            <Divider sx={{ my: 3 }} />
            <Button
              color="error"
              startIcon={<DeleteOutlined />}
              disabled={update.isPending || remove.isPending}
              onClick={() => setConfirm(true)}
            >
              アイテムを削除
            </Button>
          </CardContent>
        </Card>
      )}
      <Dialog
        open={confirm}
        onClose={() => !remove.isPending && setConfirm(false)}
      >
        <DialogTitle>アイテムを削除しますか？</DialogTitle>
        <DialogContent>削除したアイテムは復元できません。</DialogContent>
        <DialogActions>
          <Button disabled={remove.isPending} onClick={() => setConfirm(false)}>
            キャンセル
          </Button>
          <Button
            color="error"
            disabled={remove.isPending}
            onClick={() => remove.mutate()}
          >
            削除する
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  )
}

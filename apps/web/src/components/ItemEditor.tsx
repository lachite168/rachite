import { useState, type FormEvent } from 'react'
import { Button, Checkbox, Stack, TextField, Typography } from '@mui/material'
import type { Item, ItemInput } from '@/api'
export default function ItemEditor({
  initial,
  onSave,
  pending
}: {
  initial?: Item
  onSave: (input: ItemInput) => void
  pending: boolean
}) {
  const [title, setTitle] = useState(initial?.title ?? '')
  const [description, setDescription] = useState(initial?.description ?? '')
  const [completed, setCompleted] = useState(initial?.completed ?? false)
  return (
    <Stack
      component="form"
      spacing={3}
      onSubmit={(event: FormEvent) => {
        event.preventDefault()
        onSave({ title: title.trim(), description, completed })
      }}
      sx={{ pt: 1 }}
    >
      <TextField
        label="タイトル"
        value={title}
        onChange={e => setTitle(e.target.value)}
        required
        slotProps={{ htmlInput: { maxLength: 200 } }}
        autoFocus
      />
      <TextField
        label="説明"
        multiline
        minRows={4}
        value={description}
        onChange={e => setDescription(e.target.value)}
        slotProps={{ htmlInput: { maxLength: 5000 } }}
      />
      <Stack direction="row" sx={{ alignItems: 'center' }}>
        <Checkbox
          checked={completed}
          onChange={e => setCompleted(e.target.checked)}
          slotProps={{ input: { 'aria-label': '完了' } }}
        />
        <Typography>完了にする</Typography>
      </Stack>
      <Button
        type="submit"
        variant="contained"
        disabled={pending || !title.trim()}
      >
        {pending ? '保存中…' : '保存する'}
      </Button>
    </Stack>
  )
}

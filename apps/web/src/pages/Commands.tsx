import { useQuery } from '@tanstack/react-query'
import { Box, Stack, Typography } from '@mui/material'

import { api } from '@/api'
import { CommandList } from '@/components/CommandList'
import { QueryFeedback } from '@/components/QueryFeedback'

export default function CommandsPage() {
  const query = useQuery({
    queryKey: ['commands'],
    queryFn: ({ signal }) => api.commands(signal)
  })

  return (
    <Stack spacing={3}>
      <div>
        <Typography component="h1" variant="h4" gutterBottom>
          コマンドの使い方
        </Typography>
        {query.data && (
          <Typography color="text.secondary">
            traQ で <Box component="code">{query.data.mention}</Box>{' '}
            をメンションし、続けてコマンドを入力します。
          </Typography>
        )}
      </div>
      <QueryFeedback
        pending={query.isPending}
        loadingMessage="コマンドを読み込んでいます…"
        error={query.error?.message}
        retrying={query.isFetching}
        onRetry={() => void query.refetch()}
      />
      {query.data && <CommandList {...query.data} />}
    </Stack>
  )
}

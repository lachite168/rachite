import { useInfiniteQuery } from '@tanstack/react-query'
import { Alert, Button, Stack, Typography } from '@mui/material'

import { api } from '@/api'
import { QueryFeedback } from '@/components/QueryFeedback'
import { SenryuCard } from '@/components/SenryuCard'

export default function SenryuPage() {
  const query = useInfiniteQuery({
    queryKey: ['senryu'],
    queryFn: ({ pageParam, signal }) => api.senryu(pageParam, signal),
    initialPageParam: null as string | null,
    getNextPageParam: page => page.next_cursor ?? undefined
  })
  const entries = query.data?.pages.flatMap(page => page.items) ?? []

  function retry() {
    if (query.isFetchNextPageError) {
      void query.fetchNextPage()
    } else {
      void query.refetch()
    }
  }

  return (
    <Stack spacing={3}>
      <div>
        <Typography component="h1" variant="h4" gutterBottom>
          川柳一覧
        </Typography>
        <Typography color="text.secondary">
          traQ で Bot が見つけた川柳を新着順に掲載しています。
        </Typography>
      </div>
      <QueryFeedback
        pending={query.isPending}
        loadingMessage="川柳を読み込んでいます…"
        error={query.error?.message}
        retrying={query.isFetching}
        onRetry={retry}
      />
      {query.isSuccess && entries.length === 0 && (
        <Alert severity="info">まだ掲載されている川柳はありません。</Alert>
      )}
      {entries.length > 0 && (
        <Stack component="ul" spacing={2} sx={{ listStyle: 'none', p: 0 }}>
          {entries.map(entry => (
            <SenryuCard key={entry.message_id} entry={entry} />
          ))}
        </Stack>
      )}
      {query.hasNextPage && !query.isFetchNextPageError && (
        <Button
          variant="outlined"
          sx={{ alignSelf: 'center', color: 'primary.dark' }}
          disabled={query.isFetching}
          onClick={() => void query.fetchNextPage()}
        >
          {query.isFetchingNextPage ? '読み込み中…' : 'さらに読み込む'}
        </Button>
      )}
    </Stack>
  )
}

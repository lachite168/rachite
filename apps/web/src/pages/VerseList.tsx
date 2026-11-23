import { useInfiniteQuery } from '@tanstack/react-query'
import { Alert, Button, Stack, Typography } from '@mui/material'

import { api, type VerseKind } from '@/api'
import { QueryFeedback } from '@/components/QueryFeedback'
import { VerseCard } from '@/components/VerseCard'

export default function VerseList({ kind }: { kind: VerseKind }) {
  const label = kind === 'senryu' ? '川柳' : '最上川'
  const query = useInfiniteQuery({
    queryKey: [kind],
    queryFn: ({ pageParam, signal }) => api.verses(kind, pageParam, signal),
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
          {label}一覧
        </Typography>
        <Typography color="text.secondary">
          traQ で Bot が見つけた{label}を新着順に掲載しています。
        </Typography>
      </div>
      <QueryFeedback
        pending={query.isPending}
        loadingMessage={`${label}を読み込んでいます…`}
        error={query.error?.message}
        retrying={query.isFetching}
        onRetry={retry}
      />
      {query.isSuccess && entries.length === 0 && (
        <Alert severity="info">まだ掲載されている{label}はありません。</Alert>
      )}
      {entries.length > 0 && (
        <Stack component="ul" spacing={2} sx={{ listStyle: 'none', p: 0 }}>
          {entries.map(entry => (
            <VerseCard key={entry.message_id} entry={entry} />
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

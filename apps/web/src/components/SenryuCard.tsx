import { Card, CardContent, Link, Stack, Typography } from '@mui/material'

import type { Senryu } from '@/api'
import dayjs from '@/lib/date'

export function SenryuCard({ entry }: { entry: Senryu }) {
  return (
    <Card component="li">
      <CardContent>
        <Typography
          component="blockquote"
          sx={{
            m: 0,
            mb: 2,
            fontSize: '1.2rem',
            lineHeight: 1.8,
            whiteSpace: 'pre-wrap',
            overflowWrap: 'anywhere'
          }}
        >
          {entry.verse}
        </Typography>
        <Stack direction="row" spacing={2} useFlexGap sx={{ flexWrap: 'wrap' }}>
          <Typography variant="body2" color="text.secondary">
            <time dateTime={entry.detected_at}>
              {dayjs(entry.detected_at).format('YYYY/MM/DD HH:mm')}
            </time>
          </Typography>
          <Link variant="body2" href={entry.source_url} color="primary.dark">
            traQ の元投稿を開く
          </Link>
        </Stack>
      </CardContent>
    </Card>
  )
}

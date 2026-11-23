import { Box, Paper, Typography } from '@mui/material'

import type { CommandCatalog } from '@/api'

export function CommandList({ mention, commands }: CommandCatalog) {
  return (
    <Paper variant="outlined">
      <Box component="dl" sx={{ m: 0 }}>
        {commands.map(({ usage, description }) => (
          <Box
            key={usage}
            sx={{
              px: { xs: 2, sm: 3 },
              py: 2,
              '& + &': { borderTop: 1, borderColor: 'divider' }
            }}
          >
            <Typography
              component="dt"
              sx={{ fontWeight: 700, overflowWrap: 'anywhere' }}
            >
              <code>{usage ? mention + ' ' + usage : mention}</code>
            </Typography>
            <Typography
              component="dd"
              variant="body2"
              color="text.secondary"
              sx={{ mt: 0.5, ml: 0 }}
            >
              {description}
            </Typography>
          </Box>
        ))}
      </Box>
    </Paper>
  )
}

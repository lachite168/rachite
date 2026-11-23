import {
  Alert,
  Button,
  CircularProgress,
  Stack,
  Typography
} from '@mui/material'

interface QueryFeedbackProps {
  pending: boolean
  loadingMessage: string
  error?: string
  retrying: boolean
  onRetry: () => void
}

export function QueryFeedback({
  pending,
  loadingMessage,
  error,
  retrying,
  onRetry
}: QueryFeedbackProps) {
  if (pending) {
    return (
      <Stack
        role="status"
        direction="row"
        spacing={1.5}
        sx={{ alignItems: 'center' }}
      >
        <CircularProgress size={18} aria-hidden="true" />
        <Typography color="text.secondary">{loadingMessage}</Typography>
      </Stack>
    )
  }

  if (error) {
    return (
      <Alert
        severity="error"
        action={
          <Button color="inherit" disabled={retrying} onClick={onRetry}>
            {retrying ? '再試行中…' : '再試行'}
          </Button>
        }
      >
        {error}
      </Alert>
    )
  }

  return null
}

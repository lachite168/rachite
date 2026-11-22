import {
  Box,
  Button,
  Card,
  CardContent,
  Stack,
  Typography
} from '@mui/material'
export default function About() {
  return (
    <Box sx={{ maxWidth: 760 }}>
      <Typography variant="overline" color="primary">
        BUILT TO BUILD
      </Typography>
      <Typography variant="h3" sx={{ mb: 3 }}>
        次の一歩を、すぐに。
      </Typography>
      <Typography color="text.secondary" sx={{ mb: 4 }}>
        Rust と React
        で作る、フルスタック開発のスタート地点です。認証からデータの保存まで、ひとつながりのサンプルを用意しています。
      </Typography>
      <Stack spacing={2}>
        {[
          [
            'Frontend',
            'React 19 / TypeScript / Material UI / TanStack Query / Zustand'
          ],
          ['Backend', 'Axum / Tokio / SQLx / SQLite / OpenAPI'],
          [
            'Developer experience',
            'npm workspaces / mise / oxlint / Prettier / Docker'
          ]
        ].map(([title, text]) => (
          <Card key={title}>
            <CardContent>
              <Typography variant="h6" sx={{ mb: 1 }}>
                {title}
              </Typography>
              <Typography color="text.secondary">{text}</Typography>
            </CardContent>
          </Card>
        ))}
      </Stack>
      <Button
        component="a"
        href="/swagger-ui/"
        target="_blank"
        rel="noreferrer"
        sx={{ mt: 3 }}
      >
        Swagger UI を開く
      </Button>
    </Box>
  )
}

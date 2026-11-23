import { Link as RouterLink, Route, Routes } from 'react-router-dom'
import { Alert, Container, Link } from '@mui/material'

import { SiteHeader } from '@/components/SiteHeader'
import CommandsPage from '@/pages/Commands'
import SenryuPage from '@/pages/Senryu'

export default function App() {
  return (
    <>
      <SiteHeader />
      <Container
        component="main"
        id="main-content"
        tabIndex={-1}
        maxWidth="md"
        sx={{ py: { xs: 4, sm: 6 } }}
      >
        <Routes>
          <Route path="/" element={<></>} />
          <Route path="/senryu" element={<SenryuPage />} />
          <Route path="/commands" element={<CommandsPage />} />
          <Route
            path="*"
            element={
              <Alert severity="info">
                ページが見つかりません。{' '}
                <Link component={RouterLink} to="/senryu" color="primary.dark">
                  川柳一覧に戻る
                </Link>
              </Alert>
            }
          />
        </Routes>
      </Container>
    </>
  )
}

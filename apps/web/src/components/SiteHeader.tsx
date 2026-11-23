import { Link as RouterLink, NavLink } from 'react-router-dom'
import {
  AppBar,
  Button,
  Container,
  IconButton,
  Link,
  Stack,
  Toolbar,
  Tooltip
} from '@mui/material'
import { DarkModeOutlined, LightModeOutlined } from '@mui/icons-material'

import { usePreferences } from '@/store'

const navigation = [
  { path: '/senryu', label: '川柳一覧' },
  { path: '/commands', label: '使い方' }
]

export function SiteHeader() {
  const { mode, toggle } = usePreferences()
  const dark = mode === 'dark'
  const themeAction = dark ? 'ライトモードに切り替え' : 'ダークモードに切り替え'

  return (
    <>
      <Link
        href="#main-content"
        sx={{
          position: 'absolute',
          zIndex: 'appBar',
          top: 8,
          left: 8,
          px: 1.5,
          py: 1,
          bgcolor: 'background.paper',
          color: 'text.primary',
          transform: 'translateY(-150%)',
          '&:focus': { transform: 'none' }
        }}
      >
        本文へ移動
      </Link>
      <AppBar
        position="static"
        color="inherit"
        elevation={0}
        sx={{
          bgcolor: 'background.paper',
          borderBottom: 1,
          borderColor: 'divider'
        }}
      >
        <Container maxWidth="md">
          <Toolbar
            disableGutters
            sx={{ gap: { xs: 0.5, sm: 2 }, flexWrap: 'wrap', py: 1 }}
          >
            <Link
              component={RouterLink}
              to="/"
              color="text.primary"
              underline="none"
              sx={{ fontSize: '1.25rem', fontWeight: 800, mr: 'auto' }}
            >
              rachite
            </Link>
            <Stack
              component="nav"
              aria-label="メインメニュー"
              direction="row"
              spacing={0.5}
            >
              {navigation.map(({ path, label }) => (
                <Button
                  key={path}
                  component={NavLink}
                  to={path}
                  end
                  color="inherit"
                  sx={{
                    '&[aria-current="page"]': {
                      bgcolor: 'action.selected',
                      color: 'primary.dark',
                      boxShadow: theme =>
                        `inset 0 -2px 0 ${theme.palette.primary.main}`
                    }
                  }}
                >
                  {label}
                </Button>
              ))}
            </Stack>
            <Tooltip title={themeAction}>
              <IconButton
                onClick={toggle}
                aria-label={themeAction}
                color="inherit"
              >
                {dark ? <LightModeOutlined /> : <DarkModeOutlined />}
              </IconButton>
            </Tooltip>
          </Toolbar>
        </Container>
      </AppBar>
    </>
  )
}

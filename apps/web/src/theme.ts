import { createTheme, type PaletteMode } from '@mui/material/styles'

const accent = '#0485F7'
const accentForeground = '#0A1725'

const colors = {
  light: {
    background: '#F5F5F5',
    surface: '#FFFFFF',
    text: '#18181B',
    muted: '#64646E',
    border: '#E4E4E7',
    hover: '#EBEBEE',
    selected: '#E5F2FF',
    accentText: '#005EAE'
  },
  dark: {
    background: '#070709',
    surface: '#18181B',
    text: '#FCFCFC',
    muted: '#A1A1AA',
    border: '#2A2A2D',
    hover: '#27272A',
    selected: '#122A42',
    accentText: '#7DC1FF'
  }
}

export function createRachiteTheme(mode: PaletteMode) {
  const palette = colors[mode]
  return createTheme({
    palette: {
      mode,
      primary: {
        main: accent,
        dark: palette.accentText,
        contrastText: accentForeground
      },
      background: { default: palette.background, paper: palette.surface },
      text: { primary: palette.text, secondary: palette.muted },
      divider: palette.border,
      action: { hover: palette.hover, selected: palette.selected }
    },
    typography: {
      fontFamily: '"Noto Sans JP", system-ui, sans-serif',
      h4: { fontWeight: 700 },
      button: { textTransform: 'none', fontWeight: 650 }
    },
    shape: { borderRadius: 12 },
    components: {
      MuiCssBaseline: {
        styleOverrides: {
          '::selection': { backgroundColor: accent, color: accentForeground },
          'a:focus-visible, button:focus-visible': {
            outline: `2px solid ${accent}`,
            outlineOffset: 3
          }
        }
      },
      MuiButton: { defaultProps: { disableElevation: true } },
      MuiCard: { defaultProps: { variant: 'outlined' } }
    }
  })
}

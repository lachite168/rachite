import { fileURLToPath, URL } from 'node:url'

import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

import { getApiProxyTarget } from './dev.config.ts'

export default defineConfig(({ mode }) => {
  const target = getApiProxyTarget(mode)
  const proxy = {
    '^/api(?=/|\\?|$)': {
      target,
      changeOrigin: true,
      rewrite: (path: string) => path.replace(/^\/api\/?/, '/')
    },
    '/swagger-ui': { target, changeOrigin: true }
  }

  return {
    plugins: [react()],
    build: {
      rolldownOptions: {
        output: {
          codeSplitting: {
            groups: [
              {
                name: 'react-vendor',
                test: /node_modules[\\/](react|react-dom|react-router|react-router-dom)[\\/]/
              }
            ]
          }
        }
      }
    },
    resolve: {
      alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) }
    },
    server: {
      port: 5173,
      strictPort: true,
      proxy
    },
    preview: {
      host: '127.0.0.1',
      port: 4173,
      strictPort: true,
      proxy
    }
  }
})

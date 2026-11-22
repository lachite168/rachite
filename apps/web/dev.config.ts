import { fileURLToPath } from 'node:url'

import { loadEnv } from 'vite'

const workspaceRoot = fileURLToPath(new URL('../../', import.meta.url))

export function getApiProxyTarget(mode: string) {
  const env = loadEnv(mode, workspaceRoot, ['API_PROXY_TARGET', 'PORT'])
  return env['API_PROXY_TARGET'] || `http://127.0.0.1:${env['PORT'] || '3000'}`
}

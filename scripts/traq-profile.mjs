import { parseArgs } from 'node:util'

async function main() {
  const { values } = parseArgs({
    options: {
      bio: { type: 'string' },
      'display-name': { type: 'string' },
      help: { type: 'boolean', short: 'h' }
    }
  })
  if (values.help) {
    console.log(`使い方: node --env-file-if-exists=.env scripts/traq-profile.mjs [--bio=ひとこと] [--display-name=表示名]
引数なしでプロフィールを表示します。--bio= でひとことを空にします。
TRAQ_BOT_TOKEN が必要です。TRAQ_ORIGIN の既定値は https://q.trap.jp です。`)
    return
  }

  const { bio, 'display-name': displayName } = values
  if (bio !== undefined && [...bio].length > 1000) {
    throw new Error('ひとことは1000文字までです。')
  }
  if (displayName !== undefined && [...displayName].length > 32) {
    throw new Error('表示名は32文字までです。')
  }

  const token = process.env.TRAQ_BOT_TOKEN?.trim()
  if (!token) throw new Error('TRAQ_BOT_TOKEN を設定してください。')

  const origin = URL.parse(process.env.TRAQ_ORIGIN ?? 'https://q.trap.jp')
  if (
    !origin ||
    !['https:', 'http:'].includes(origin.protocol) ||
    origin.href !== `${origin.origin}/`
  ) {
    throw new Error(
      'TRAQ_ORIGIN には traQ の HTTP(S) オリジンを指定してください。'
    )
  }

  const url = new URL('/api/v3/users/me', origin)
  const authorization = `Bearer ${token}`
  const response = await request(url, { headers: { authorization } })
  const me = await response.json()
  if (me.bot !== true) {
    throw new Error('Bot のアクセストークンを指定してください。')
  }

  if (bio === undefined && displayName === undefined) {
    const { name, displayName, bio } = me
    console.log(JSON.stringify({ name, displayName, bio }, null, 2))
    return
  }

  await request(url, {
    method: 'PATCH',
    headers: { authorization, 'content-type': 'application/json' },
    body: JSON.stringify({ bio, displayName })
  })
  console.log(`@${me.name} のプロフィールを更新しました。`)
}

async function request(url, options) {
  const method = options.method ?? 'GET'
  const response = await fetch(url, {
    ...options,
    redirect: 'error',
    signal: AbortSignal.timeout(10_000)
  }).catch(() => {
    throw new Error(`${method} ${url.pathname} に接続できませんでした。`)
  })
  const expectedStatus = method === 'PATCH' ? 204 : 200
  if (response.status !== expectedStatus) {
    throw new Error(
      `${method} ${url.pathname} が失敗しました（HTTP ${response.status}）。`
    )
  }
  return response
}

try {
  await main()
} catch (error) {
  console.error(error.message)
  process.exitCode = 1
}

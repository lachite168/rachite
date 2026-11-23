import assert from 'node:assert/strict'
import { test } from 'node:test'

import { api } from '../src/api.ts'

test('requests a first page and encodes an opaque continuation cursor', async t => {
  const signal = new AbortController().signal
  const cursor = 'legacy.+/=&'
  const nextPage = { items: [], next_cursor: null }
  const fetch = t.mock.method(globalThis, 'fetch', (input: string) =>
    Promise.resolve(
      Response.json(
        input === '/api/senryu'
          ? { ...nextPage, next_cursor: cursor }
          : nextPage
      )
    )
  )

  const first = await api.verses('senryu', null, signal)
  assert.equal(first.next_cursor, cursor)
  assert.deepEqual(
    await api.verses('senryu', first.next_cursor, signal),
    nextPage
  )
  assert.deepEqual(await api.verses('mogamigawa', null, signal), nextPage)
  assert.deepEqual(
    fetch.mock.calls.map(call => call.arguments),
    [
      ['/api/senryu', { signal, headers: { Accept: 'application/json' } }],
      [
        '/api/senryu?before=legacy.%2B%2F%3D%26',
        { signal, headers: { Accept: 'application/json' } }
      ],
      ['/api/mogamigawa', { signal, headers: { Accept: 'application/json' } }]
    ]
  )
})

test('requests the command catalog as JSON', async t => {
  const signal = new AbortController().signal
  const catalog = {
    mention: '@BOT_lachite',
    commands: [{ usage: 'help', description: '使い方を表示します。' }]
  }
  const fetch = t.mock.method(globalThis, 'fetch', () =>
    Promise.resolve(
      new Response(JSON.stringify(catalog), {
        headers: { 'Content-Type': 'application/json; charset=utf-8' }
      })
    )
  )

  assert.deepEqual(await api.commands(signal), catalog)
  assert.deepEqual(fetch.mock.calls[0]?.arguments, [
    '/api/commands',
    { signal, headers: { Accept: 'application/json' } }
  ])
})

test('reports authentication, server, and non-JSON responses', async t => {
  for (const [status, message] of [
    [401, '閲覧には部員認証が必要です。'],
    [403, '閲覧には部員認証が必要です。'],
    [503, 'データを取得できませんでした。'],
    [200, 'API の応答を読み込めませんでした。']
  ] as const) {
    await t.test(`HTTP ${status}`, async t => {
      t.mock.method(globalThis, 'fetch', () =>
        Promise.resolve(
          new Response('<p>HTML response</p>', {
            status,
            headers: { 'Content-Type': 'text/html' }
          })
        )
      )

      await assert.rejects(api.commands(new AbortController().signal), {
        message
      })
    })
  }
})

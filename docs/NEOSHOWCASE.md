# NeoShowcase へのデプロイと Bot の運用

Web と API・Bot を同じ Git リポジトリから別々のアプリとしてデプロイします。以下は `rachite.trap.show` の設定例です。サブドメインを変える場合は、すべての公開 URL で揃えます。

## リポジトリとアプリを登録

変更を GitHub に push し、NeoShowcase の Repositories で `lachite168/rachite` を登録します。非公開リポジトリでデプロイキーが表示された場合は、GitHub の Settings → Deploy keys に読み取り用として追加します。

| 項目                 | Frontend                 | Backend          |
| -------------------- | ------------------------ | ---------------- |
| Application Name     | `rachite-web`            | `rachite`        |
| Branch               | 変更を push したブランチ | Frontend と同じ  |
| Deploy Type          | Static                   | Runtime          |
| Build Type           | Dockerfile               | Dockerfile       |
| Context              | `.`                      | `.`              |
| Dockerfile Name      | `Dockerfile`             | `Dockerfile.api` |
| Artifact Path        | `/usr/share/nginx/html`  | なし             |
| SPA                  | 有効                     | なし             |
| Use Database         | No                       | Yes、MariaDB     |
| Entrypoint / Command | なし                     | 空欄             |

## URLs

各 URL で `https`、サブドメイン `rachite`、ドメイン `.trap.show` を選びます。Port Forwarding は空欄にします。

| アプリ   | 公開パス      | パス欄の値   | 転送先       | Strip Prefix | 認証 |
| -------- | ------------- | ------------ | ------------ | ------------ | ---- |
| Frontend | `/`           | 空欄         | 静的ファイル | 無効         | Hard |
| Backend  | `/api`        | `api`        | `3000/TCP`   | 有効         | Hard |
| Backend  | `/swagger-ui` | `swagger-ui` | `3000/TCP`   | 無効         | Hard |

`/api/health` は Axum の `/health` に転送されます。Swagger UI は同じホストの `/api/openapi.json` を読み込みます。SPA を有効にすると `/senryu`、`/mogamigawa`、`/commands` を直接開けます。

## Backend の環境変数

| キー               | 値                                            |
| ------------------ | --------------------------------------------- |
| `RACHITE_DATABASE` | `neoshowcase`                                 |
| `HOST`             | `0.0.0.0`                                     |
| `PORT`             | `3000`                                        |
| `TRAQ_BOT_ID`      | Bot Console の Bot ID                         |
| `TRAQ_BOT_TOKEN`   | 同じ Bot の Access Token                      |
| `TRAQ_BOT_NAME`    | メンション名。省略時は `BOT_lachite`          |
| `TRAQ_ORIGIN`      | 省略時は `https://q.trap.jp`                  |
| `RUST_LOG`         | 任意。例は `rachite_api=info,tower_http=info` |

API は NeoShowcase が設定する `NS_MARIADB_HOSTNAME`、`NS_MARIADB_PORT`、`NS_MARIADB_DATABASE`、`NS_MARIADB_USER`、`NS_MARIADB_PASSWORD` を使って接続し、起動時にマイグレーションを適用します。Backend の `DATABASE_URL` と Frontend の環境変数は空欄で構いません。

## Bot の設定と切り替え

Bot Console で対象 Bot を WebSocket Mode にし、次のイベントを購読します。 [Bot Console](https://bot-console.trap.jp/)・[traQ のイベント一覧](https://github.com/traPtitech/traQ-bot-console/blob/dev/src/docs/bot/events/message.md)

| イベント                                     | 用途                                |
| -------------------------------------------- | ----------------------------------- |
| `MESSAGE_CREATED`、`MENTION_MESSAGE_CREATED` | チャンネル投稿と Bot 宛てメンション |
| `MESSAGE_UPDATED`、`MESSAGE_DELETED`         | 川柳・最上川の一覧更新              |
| `DIRECT_MESSAGE_CREATED`                     | Bot との DM での `markov`           |

Web と API だけを確認する間は、Bot ID と Access Token を両方空欄にできます。テスト用 Bot で一通り確認したら、旧 Bot を停止して本番用の設定に切り替えます。本番の Backend は1インスタンスで稼働させます。

## デプロイ後の確認

1. 両アプリのビルドログと、Backend の `API ready` を確認します。
2. 未認証で Web、`/api/senryu`、`/swagger-ui/` にアクセスし、Hard 認証が求められることを確認します。Backend のポートへ未認証で直接到達できないことも確認します。
3. 認証後に `/senryu`、`/mogamigawa`、`/commands`、`/swagger-ui/` を直接開きます。`/api/health` が `{"status":"ok"}` を返すことを確認します。
4. テスト Bot の `/api/bot/status` が `{"status":"connected"}` を返すことを確認します。チャンネルで `ping` と `join` を試します。 [Bot 接続の切り分け](handout.md#bot-接続の確認)
5. テスト用チャンネルと Bot との DM で `markov <username>` と `markov <username> --here` を試します。検索範囲と返信先を確認します。
6. 川柳・最上川をオンにしたチャンネルで通常の投稿、編集、削除を試し、Web 一覧への反映を確認します。最上川の返信と3種のスタンプ、引用リンクを使った少数回の `stampbomb` も確認します。 [機能仕様](specs.md)

## Bot のプロフィールを確認・更新する

リポジトリのルートで実行します。`.env` または環境変数の `TRAQ_BOT_TOKEN` が指す Bot を操作します。`TRAQ_ORIGIN` の既定値は `https://q.trap.jp` です。

```bash
# 現在の表示名とひとことを表示
node --env-file-if-exists=.env scripts/traq-profile.mjs

# ひとことを更新
node --env-file-if-exists=.env scripts/traq-profile.mjs --bio="川柳を探しています"

# ひとことを空にし、表示名も更新
node --env-file-if-exists=.env scripts/traq-profile.mjs --bio= --display-name="らちて"
```

# NeoShowcase へのデプロイ

Web と API・Bot を同じ Git リポジトリから別々のアプリとしてデプロイします。ここでは `rachite.trap.show` を使います。サブドメインを変える場合は、すべての URL に同じサブドメインを設定します。

## リポジトリとアプリを登録

変更をコミットして GitHub に push し、NeoShowcase の Repositories で `lachite168/rachite` を登録します。非公開リポジトリでデプロイキーが表示された場合は、GitHub の Settings → Deploy keys に読み取り用として追加します。

同じリポジトリから、次の設定でアプリを作成します。

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

`/api/health` は Axum の `/health` に転送されます。Swagger UI のブラウザは同じホストの `/api/openapi.json` を読み込みます。SPA を有効にすると `/senryu` と `/commands` を直接開けます。

## Backend の Environment Variables

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

アプリは NeoShowcase が設定する `NS_MARIADB_HOSTNAME`、`NS_MARIADB_PORT`、`NS_MARIADB_DATABASE`、`NS_MARIADB_USER`、`NS_MARIADB_PASSWORD` で接続し、起動時にマイグレーションを適用します。Backend の `DATABASE_URL` と Frontend の環境変数は空欄で構いません。ローカルと Compose では引き続き既存の DB 設定を使います。

Bot を起動する前に [Bot Console](https://bot-console.trap.jp/) で WebSocket Mode を選び、`MESSAGE_CREATED`、`MENTION_MESSAGE_CREATED`、`MESSAGE_UPDATED`、`MESSAGE_DELETED` を購読します。Access Token は NeoShowcase の環境変数に登録します。Web と API を先に確認する場合は、Bot ID と Access Token を両方未設定にして起動できます。本番の Bot ID に切り替えるときは、旧 Bot を停止してから Backend を1インスタンスで常時稼働させます。

## デプロイ後の確認

1. 両方のアプリをデプロイし、ビルドログと Backend の `API ready` を確認します。
2. 未認証のブラウザで `/senryu`、`/api/senryu`、`/swagger-ui/` を開き、認証が求められることを確認します。
3. 認証後に `/senryu`、`/commands`、`/swagger-ui/` を直接開き、`/api/health` が `{"status":"ok"}` を返すことを確認します。
4. Bot の有効化後は `/api/bot/status` が `{"status":"connected"}` を返すことを確認し、テスト用チャンネルで Bot に `ping` を送ります。

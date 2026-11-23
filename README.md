# rachite

traQ Bot と、Bot が見つけた川柳を閲覧する部内向け Web です。Web は React + MUI、HTTP API と Bot は Rust（Axum・Tokio）、DB は MariaDB を使います。ユーザー向け仕様は [docs/specs.md](docs/specs.md)、実装時の参照先は [docs/handout.md](docs/handout.md) にあります。

## ローカルで起動

WSL などの開発環境に Node.js（`.node-version`）、Rust stable、Docker Compose を用意します。Rust を初めてビルドする際には Lindera が IPADIC 辞書をダウンロードします。

```bash
mise trust
mise run init
cp .env.example .env
mise run dev
```

- `mise run dev` は Compose の開発用 MariaDB を起動し、準備完了を待ってから API と Web を起動します。既に稼働している MariaDB を使う場合は `.env` の `DATABASE_URL` を設定し、`mise run dev --no-compose-db` を実行します。
- 川柳一覧は [http://localhost:5173/senryu](http://localhost:5173/senryu)、コマンド案内は [http://localhost:5173/commands](http://localhost:5173/commands) です。Vite が `/api` と `/swagger-ui` を Axum に転送します。
- DB は `.env` の `DATABASE_URL` に接続し、API 起動時にマイグレーションを適用します。既定ではローカルの Compose DB を使います。
- Bot の動作確認には、テスト用 Bot の `TRAQ_BOT_ID` と `TRAQ_BOT_TOKEN`、必要に応じて `TRAQ_BOT_NAME` を設定します。ID とトークンの両方を空にすると、Web と API だけを開発できます。
- 川柳検出は、誰でもコマンドを送ったチャンネルで `@BOT_lachite senryu on` / `off` により切り替えられます。`senryu status` で現在の状態を確認できます。

Compose で全体を起動する場合は `mise run up` を実行します。川柳一覧は [http://localhost:8080/senryu](http://localhost:8080/senryu) で確認できます。Compose 内の Axum には nginx が接続します。DB のパスワードに URL の予約文字を使う場合は、URL エンコードした `DATABASE_URL` と `COMPOSE_DATABASE_URL` を設定してください。

## Bot のプロフィールを確認・更新する

リポジトリのルートで実行します。既存の `.env` または環境変数に設定した `TRAQ_BOT_TOKEN` の Bot が対象です。`TRAQ_ORIGIN` の既定値は `https://q.trap.jp` です。

```bash
# 現在の表示名とひとことを表示
node --env-file-if-exists=.env scripts/traq-profile.mjs

# ひとことを更新
node --env-file-if-exists=.env scripts/traq-profile.mjs --bio="川柳を探しています"

# ひとことを空にし、表示名も更新
node --env-file-if-exists=.env scripts/traq-profile.mjs --bio= --display-name="らちて"
```

## 確認

| コマンド                                               | 用途                                                              |
| ------------------------------------------------------ | ----------------------------------------------------------------- |
| `mise run check`                                       | Web の型、lint と整形、通常の Rust テストを確認                   |
| `mise run build`                                       | Web と API を本番向けにビルド                                     |
| `cargo test --workspace --locked -- --include-ignored` | `TEST_DATABASE_URL` に設定したテスト専用の MariaDB を含めてテスト |

テスト用 DB は空のデータベースを作成し、接続ユーザーにその DB の DDL と読み書きを許可します。GitHub Actions の API ジョブは MariaDB を起動してテストします。

`/api/health` は DB への接続、`/api/bot/status` は Bot の状態（`disabled`、`disconnected`、`connected`）を返します。`/swagger-ui/` から API を確認できます。

## NeoShowcase で部内公開

Web を Static、API と traQ Bot を Runtime として同じドメインに配置し、NeoShowcase の MariaDB を使います。登録画面に入力する値と Bot の切り替えは [NeoShowcase へのデプロイ](docs/NEOSHOWCASE.md) を参照してください。

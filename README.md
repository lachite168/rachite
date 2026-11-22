# Fullstack Starter

Rust (Axum + SQLx + SQLite) と React / TypeScript のモノレポ型スターターです。
登録・ログイン、ユーザー別のアイテム CRUD、ダークモード、Swagger UI までを一通りつないでいます。

## 技術構成

| 領域             | 構成                                                                   |
| ---------------- | ---------------------------------------------------------------------- |
| フロントエンド   | React 19 / TypeScript 7 / Vite 8 / MUI 9 + Emotion                     |
| 状態・ルート     | TanStack Query 5 / Zustand 5 / React Router 7                          |
| 通知・日時       | notistack 3 / dayjs（日本語・相対日時）                                |
| API              | Axum 0.8 / Tokio 1 / tower-http 0.7                                    |
| データベース     | SQLite / SQLx 0.9 / 起動時マイグレーション / WAL                       |
| 認証             | Argon2id / JWT HS256（1時間） / UUID v7                                |
| その他           | thiserror / anyhow / reqwest 0.13 / chrono / dotenvy / tracing         |
| API ドキュメント | utoipa 6 / utoipa-swagger-ui 10（vendored）                            |
| 開発             | npm workspaces / mise / oxlint / Prettier / axum-test / GitHub Actions |
| コンテナ         | Docker マルチステージ / nginx / Compose / SQLite 永続ボリューム        |

依存関係の正確な解決結果は `package-lock.json` と `Cargo.lock` に固定しています。
Axum 0.8 と Swagger UI 10 の互換性を優先しています。reqwest は共有のタイムアウト付きクライアントとして `AppState.http` に用意しています。

## クイックスタート

Node.js は [`.node-version`](.node-version) で指定し、同じ版を mise、GitHub Actions、Docker で使います。Rust stable（edition 2024 対応）、C コンパイラ、CMake も必要です。
Windows では WSL 上で Node.js と Rust の両方を実行する構成を推奨します。

```bash
# 初回は設定内容を確認してから実行
mise trust
mise install
mise run init
cp .env.example .env
# .env の JWT_SECRET を以下で生成した値に置換してください
openssl rand -hex 32

mise run dev
```

mise を使わない場合は、対応する Node.js・npm・Rust を用意して `npm ci` を実行します。上と同じ手順で `.env` を作成し、`JWT_SECRET` を設定してから `npm run dev` で起動します。

- Web: [http://localhost:5173](http://localhost:5173)
- API health: [http://localhost:3000/api/health](http://localhost:3000/api/health)
- Swagger UI: [http://localhost:3000/swagger-ui/](http://localhost:3000/swagger-ui/)
- OpenAPI: [http://localhost:3000/api/openapi.json](http://localhost:3000/api/openapi.json)

最初に画面の「アカウントを新規作成」から登録してください。初期ユーザー・初期パスワードはありません。
JWT_SECRET の未設定・短い値・サンプル値は起動時に拒否します。
ルートで起動すると SQLite の `app.db` が自動生成され、マイグレーションが適用されます。
`DATABASE_URL` の相対パスは実行ディレクトリを基準にします。環境変数の例は `.env.example` を参照してください。

## ディレクトリ

```text
.
├── apps
│   ├── api
│   │   ├── src          # API、認証、エラー、アイテム
│   │   ├── migrations   # バージョン管理する SQL
│   │   └── tests        # インメモリ SQLite の統合テスト
│   └── web
│       └── src          # pages / components / lib / API / Zustand
├── infra/nginx.conf     # SPA fallback / API reverse proxy
├── Cargo.toml           # Cargo workspace
├── package.json         # npm workspaces / 共通タスク
├── mise.toml
├── .node-version
├── Dockerfile
└── compose.yaml
```

フロントエンドは `@/*` → `apps/web/src/*` のパスエイリアスを使えます。
Vite の開発サーバーとプレビューは `/api` と `/swagger-ui` を API に転送します。
既定の転送先は `http://127.0.0.1:3000` です。ルートの `.env` の `PORT` を変更すると転送先にも反映されます。別の API に接続する場合は `API_PROXY_TARGET` を指定できます。
画面ルートは `/`、`/items/:id`、`/about` です。

## コマンド

| コマンド                               | 用途                                                        |
| -------------------------------------- | ----------------------------------------------------------- |
| `mise run help`                        | 開発タスクの一覧                                            |
| `mise run init`                        | Web の依存をインストールし、Rust の依存を取得               |
| `mise run dev` / `npm run dev`         | API と Web を同時起動                                       |
| `npm run dev:api` / `npm run dev:web`  | 個別起動                                                    |
| `npm run type-check`                   | TypeScript の型検査（`typecheck` も利用可）                 |
| `npm run lint:no-fix` / `npm run lint` | Oxlint と Clippy でコードを検査                             |
| `npm run lint:fix`                     | Web を Oxlint で自動修正                                    |
| `mise run format` / `npm run format`   | Prettier と rustfmt で整形                                  |
| `npm run format:check`                 | 整形差分を確認                                              |
| `mise run test` / `npm test`           | API 統合テスト                                              |
| `mise run check`                       | 型・lint・整形・テストを一括実行                            |
| `mise run build` / `npm run build`     | Web と API の本番ビルド                                     |
| `npm run preview`                      | ビルド済みの Web を localhost:4173 で確認（API は別途起動） |
| `mise run clean`                       | Rust・Web の生成物とインストール済みの npm 依存を削除       |
| `mise run up` / `mise run down`        | Compose で起動・停止                                        |

削除予定は `mise run clean --dry-run` で確認できます。対象は `target/`、ルートと Web の `node_modules/`・`dist/`、Web の TypeScript ビルドキャッシュです。
`.env`、SQLite の DB、lockfile、Docker のボリュームは対象外です。クリーン後に開発を再開するには `mise run init` を実行してください。

`.npmrc` は依存追加時にバージョンを完全指定で保存し、npm のインストールスクリプトを無効にします。
依存の解決では公開から7日以上経ったバージョンを選び、peer dependency の不整合はエラーにします。
Node.js を更新する際は `.node-version`、`package.json` の対応範囲、Dockerfile の Node.js イメージを合わせて確認してください。

GitHub Actions はメインブランチへの push とプルリクエストで動きます。
Web の lint・整形・型検査・ビルドと、API の整形・Clippy・テスト・ビルドを別々のジョブで確認できます。
Dependabot は npm、Cargo、GitHub Actions の依存を月次で確認する設定です。
実行条件、ログの見方、バージョン更新の流れは [GitHub の設定ガイド](docs/GITHUB.md) を参照してください。

## API と認証

| Method             | Path                 | 用途                        |
| ------------------ | -------------------- | --------------------------- |
| GET                | `/api/health`        | DB 接続を含むヘルスチェック |
| POST               | `/api/auth/register` | 登録（email / password）    |
| POST               | `/api/auth/login`    | JWT 発行                    |
| GET                | `/api/auth/me`       | ログインユーザー            |
| GET / POST         | `/api/items`         | 一覧・作成                  |
| GET / PUT / DELETE | `/api/items/{id}`    | 詳細・更新・削除            |

アイテムの入力: `{ "title": "Hello", "description": "", "completed": false }`。
PUT は置換更新です。title は前後空白を除き1〜200文字、description は5000文字以内。
パスワードは12〜128バイト。メールアドレスは前後空白を除去して小文字で保存します。
認証 API のレスポンスは `{ token, user }`。以降は `Authorization: Bearer <token>` を送信します。
Swagger UI の Authorize には token のみを入力します。

トークンはブラウザのメモリにのみ保持します。再読み込み時は再ログインが必要です。
期限切れなどで401になった場合もログイン画面に戻ります。ログアウトはブラウザ内のセッションを破棄します。
テーマ設定のみ localStorage に保存します。ユーザーが切り替わると Query キャッシュを破棄します。
各 CRUD クエリは所有ユーザー ID で絞り込み、他ユーザーのアイテムは404を返します。
ドメインエラーは `{ code, message }`。不正な JSON など Axum の抽出エラーはフレームワーク標準形式です。
SQLx は実行時クエリを利用するため、ビルド時の DB / `.sqlx` メタデータ準備は不要です。

## Docker

ルートの `.env` にランダムな JWT_SECRET を設定してから実行します。

```bash
mise run up     # docker compose up -d --build
mise run down   # docker compose down
```

Web は [http://localhost:8080](http://localhost:8080)、API / Swagger は localhost:3000。
nginx が API を同一オリジンでプロキシし、画面の深い URL への直接アクセスにも対応します。
DB は `sqlite-data` ボリュームの `/data/app.db` に保存されます。
`docker compose down` で DB は保持されます。`docker compose down -v` は DB を削除します。
Docker 内の API は非 root ユーザーで動作し、SIGTERM で graceful shutdown します。
SQLite を使うため、まずは単一 API インスタンスで運用してください。

## 拡張時のポイント

このテンプレートの認証は出発点です。公開運用する場合は、用途に応じて HTTPS、ログインのレート制限、
メール確認、パスワード再設定、JWT 失効・更新の仕組みを追加してください。
JWT_SECRET を変更すると既存トークンはすべて無効になります。
一覧は全件取得のため、データ量が増えたらページネーションを追加してください。
DB をバックアップする際は稼働中の SQLite WAL を考慮し、SQLite のバックアップ API などを利用してください。

## 仕様の参照先

- [Vite の実行環境](https://vite.dev/guide/)
- [Axum のルーティング](https://docs.rs/axum/0.8.9/axum/struct.Router.html)
- [Swagger UI の Axum 統合](https://docs.rs/utoipa-swagger-ui/10.0.2/utoipa_swagger_ui/)

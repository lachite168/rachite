# rachite

traQ で川柳・最上川を検出する Bot と、検出結果やコマンドの使い方を閲覧する部内向け Web です。Web は React と MUI、API と Bot は Rust、保存には MariaDB を使います。

## ローカルで起動

WSL などの開発環境に mise、Node.js（`.node-version`）、Rust stable、Docker Compose を用意します。Rust の初回ビルドでは Lindera が SudachiDict を取得し、API のバイナリに組み込みます。初回のセットアップと起動は以下の手順です。

```bash
mise trust
mise run init
cp .env.example .env
mise run dev
```

`mise run dev` は開発用 MariaDB を Compose で起動し、準備完了を待ってから API と Web を起動します。API は起動時に DB のマイグレーションを適用します。稼働中の MariaDB を使う場合は、`.env` の `DATABASE_URL` を設定して `mise run dev --no-compose-db` を実行します。

| 用途         | ローカル URL                                                    |
| ------------ | --------------------------------------------------------------- |
| 川柳一覧     | [localhost:5173/senryu](http://localhost:5173/senryu)           |
| 最上川一覧   | [localhost:5173/mogamigawa](http://localhost:5173/mogamigawa)   |
| コマンド一覧 | [localhost:5173/commands](http://localhost:5173/commands)       |
| Swagger UI   | [localhost:5173/swagger-ui/](http://localhost:5173/swagger-ui/) |

Bot を使うときは、テスト用 Bot の `TRAQ_BOT_ID`、`TRAQ_BOT_TOKEN` と、必要に応じて `TRAQ_BOT_NAME` を `.env` に設定します。ID とトークンを両方空にすると Bot は `disabled` になり、Web と API を利用できます。 [Bot 接続の確認](docs/handout.md#bot-接続の確認)

Web・API・DB をすべて Compose で起動する場合は `mise run up` を実行し、[localhost:8080](http://localhost:8080) を開きます。DB パスワードに URL の予約文字を使う場合は、接続 URL 中のパスワードをエンコードして `DATABASE_URL` と `COMPOSE_DATABASE_URL` に設定します。

## ドキュメント

| 知りたいこと                                     | 参照先                                                    |
| ------------------------------------------------ | --------------------------------------------------------- |
| コマンドの挙動、川柳・最上川の判定、Web の表示   | [機能仕様](docs/specs.md)                                 |
| コードの入口、テスト、Bot 接続の切り分け         | [開発・保守ガイド](docs/handout.md)                       |
| NeoShowcase へのデプロイ、Bot 設定・プロフィール | [デプロイ・運用手順](docs/NEOSHOWCASE.md)                 |
| CI、Dependabot、PR                               | [GitHub の設定](docs/GITHUB.md)                           |
| 元になったサンプルの設計                         | [ひな形の要件（参考資料）](docs/TEMPLATE_REQUIREMENTS.md) |

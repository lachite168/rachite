# rachite 開発・保守ガイド

ローカル起動は [README](../README.md#ローカルで起動)、コマンドや検出の挙動は [機能仕様](specs.md)、Bot の購読やデプロイは [運用手順](NEOSHOWCASE.md) を参照してください。

## コードの入口

Web は React、TypeScript、Vite、MUI で構成します。Axum と traQ Bot は同じ Rust プロセスで動き、Bot は WebSocket でイベントを受け、traQ の HTTP API で履歴取得・投稿・スタンプ送信を行います。川柳と最上川は SQLx 経由で MariaDB に保存します。

| 変更する箇所                       | 主なファイル                                                                                                                                                                               |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 起動・設定・DB マイグレーション    | [main.rs](../apps/api/src/main.rs)、[config.rs](../apps/api/src/config.rs)、[マイグレーション](../apps/api/migrations/)                                                                    |
| WebSocket 受信・イベントの振り分け | [bot.rs](../apps/api/src/bot.rs)、[traq/event.rs](../apps/api/src/traq/event.rs)                                                                                                           |
| コマンドの登録・ヘルプ             | [commands.rs](../apps/api/src/bot/commands.rs)、[各コマンド](../apps/api/src/bot/commands/)                                                                                                |
| traQ API との通信                  | [client.rs](../apps/api/src/traq/client.rs)、投稿・履歴・投稿リンクは [client/messages.rs](../apps/api/src/traq/client/messages.rs)                                                        |
| 川柳・最上川                       | [判定](../apps/api/src/verse/detect.rs)、[VerseStore](../apps/api/src/verse/store.rs)、[最上川の通知](../apps/api/src/bot/mogamigawa.rs)                                                   |
| Markdown 抽出・形態素解析          | [本文抽出](../apps/api/src/text/markdown.rs)、[共用辞書](../apps/api/src/text.rs)、[川柳・最上川の読みと品詞](../apps/api/src/verse/dictionary.rs)                                         |
| Web 向け API                       | [ルートと OpenAPI](../apps/api/src/http.rs)                                                                                                                                                |
| Web の画面・通信・配色             | [ルート](../apps/web/src/App.tsx)、[画面](../apps/web/src/pages/)、[共用部品](../apps/web/src/components/)、[API クライアント](../apps/web/src/api.ts)、[テーマ](../apps/web/src/theme.ts) |

Rust のモジュールは `bot.rs` と `bot/` のように同じ名前のファイルを入口にします。コマンドは `bot/commands.rs` に登録し、各コマンドの処理を `bot/commands/` に置きます。Bot の `help` と Web 向けの `/api/commands` はその登録を参照します。

### 本文処理と保存

- 本文抽出では `pulldown-cmark` で Markdown を読み、traQ の埋め込み、スタンプ、メンション、スポイラーを処理します。川柳・最上川の判定には埋め込みの置換前の `text` を使います。 [本文抽出](../apps/api/src/text/markdown.rs)・[traQ の本文変換](https://github.com/traPtitech/traQ/blob/master/utils/message/parser.go)
- SudachiDict の読みと品詞は項目名で参照します。英字は小文字にして辞書を引き、表示には元の綴りを使います。たとえば「月は東に日は西に」は同梱辞書で一語になるため、中七の境界を確認するときは空白を入れた「菜の花や 月は東に 日は西に」と比較できます。 [辞書との接続](../apps/api/src/verse/dictionary.rs)・[判定テスト](../apps/api/src/verse/detect.rs)・[Lindera の辞書形式](https://lindera.github.io/lindera/lindera-sudachidict/dictionary_format.html)
- 既存の掲載結果を辞書変更時に一括で再判定する処理はありません。編集イベントでは解析エラーと未検出を区別します。最上川は保存と通知を分け、元投稿 ID で通知の重複を防ぎます。旧形式の最上川の行はマイグレーションで維持します。 [イベント処理](../apps/api/src/bot.rs)・[最上川の一覧用マイグレーション](../apps/api/migrations/0003_mogamigawa_listing.sql)

### コマンドの実装

- `stampgacha` の正規表現は、スタンプ一覧の取得後に QuickJS のコンパイルと照合を `spawn_blocking` で実行します。照合ごとに `lastIndex` を戻し、割り込みハンドラーで250 msの期限を確認します。通常の文字列の検索と抽選は Rust が担当します。 [コマンド](../apps/api/src/bot/commands/stampgacha.rs)・[照合処理](../apps/api/src/bot/commands/stampgacha/regexp.rs)
- `stampbomb` は引用リンクを `plainText` から取得できないとき、元の `text` の同じ引数位置を調べます。ジョブを単一ワーカーに渡し、送信の間隔はジョブの境目にも適用します。 [コマンド](../apps/api/src/bot/commands/stampbomb.rs)・[ワーカー](../apps/api/src/bot/stampbomb.rs)
- `markov` は `GET /messages` の `from` と `in` で公開・チャンネル内検索を切り替えます。DM 内限定の履歴取得と DM への返信は、実行者の UUID を使って `/users/{userId}/messages` を呼びます。履歴取得をワーカーに渡し、形態素解析と生成は `spawn_blocking` で実行します。 [ワーカー](../apps/api/src/bot/markov.rs)・[traQ v3 OpenAPI](https://github.com/traPtitech/traQ/blob/master/docs/v3-api.yaml)

## 検証

| コマンド                                               | 用途                                         |
| ------------------------------------------------------ | -------------------------------------------- |
| `mise run check`                                       | 型検査、lint、整形、Web と通常の Rust テスト |
| `mise run test`                                        | Web と通常の Rust テスト                     |
| `npm run test:web`                                     | Web のテスト                                 |
| `cargo test --workspace --locked -- --include-ignored` | MariaDB を使うテストを含む Rust テスト       |
| `mise run build`                                       | Web と API の本番ビルド                      |

DB 依存テストには、空のテスト専用 MariaDB データベースを作ります。その DB の DDL と読み書きが可能なユーザーの接続 URL を `TEST_DATABASE_URL` に設定します。CI では API ジョブが MariaDB サービスを起動します。 [CI の内訳](GITHUB.md#github-actions-と-ci)

- HTTP API の統合テストは [apps/api/tests/api.rs](../apps/api/tests/api.rs) にあります。
- 模擬 traQ を使う Bot テストは [apps/api/tests/bot/](../apps/api/tests/bot/) にまとめ、`#[path]` で内部テストとして読み込みます。コマンドの返信、キュー、チャンネルと DM の経路をここで確認します。
- 本文抽出や判定などの小さな単体テストは対象モジュールに置きます。実環境での確認項目は [デプロイ後の確認](NEOSHOWCASE.md#デプロイ後の確認) を参照してください。

## Bot 接続の確認

1. テスト用 Bot を WebSocket Mode に設定し、必要なイベントを購読します。 [Bot の設定と切り替え](NEOSHOWCASE.md#bot-の設定と切り替え)
2. `.env` の `TRAQ_BOT_ID` と `TRAQ_BOT_TOKEN` に同じ Bot の ID と Access Token を設定し、`mise run dev` を再起動します。アプリが認証ヘッダーに `Bearer ` を付けます。 [認証の実装](../apps/api/src/traq/client.rs)
3. `curl -fsS http://127.0.0.1:5173/api/bot/status` で `{"status":"connected"}` を確認します。

| 状態・エラー                             | 確認すること                                                                                                   |
| ---------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `disabled`                               | Bot を起動する場合は ID とトークンを両方設定します。                                                           |
| `disconnected` または `401 Unauthorized` | API のログを確認し、Bot Console の設定と、ID、トークン、`TRAQ_ORIGIN` の組み合わせを照合します。               |
| Vite が `/api` に接続できない            | API 側の起動ログを先に確認します。MariaDB への接続で止まる場合は DB の起動状況と `DATABASE_URL` を確認します。 |

旧 Bot の互換性を調べるときは、隣の `../test2` の `src/usecase/bot_service.rs`、`src/usecase/commands/`、`tests/test.rs` を参照できます。`RACHITE_OPERATOR_IDS` は旧版の許可リストです。現行の川柳検出はチャンネル内の `senryu status` / `on` / `off` で管理します。

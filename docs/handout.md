# rachite 開発引き継ぎ

仕様は [specs.md](specs.md)、起動手順は [README.md](../README.md) を参照してください。この文書は Codex で開発を再開するための作業状況です。

## 再開時に確認すること

2026年11月25日のマルコフ生成は `feat/markov` で進めます。別のチェックアウトで再開する場合は `git status --short --branch` でブランチと作業ツリーを確認してください。

`apps/web` は React + TypeScript + MUI、`apps/api` は Rust + Axum + Tokio です。Axum と traQ Bot は同じプロセスで起動し、SQLx 経由で MariaDB を使います。テンプレートの要件は [TEMPLATE_REQUIREMENTS.md](TEMPLATE_REQUIREMENTS.md) に記録されています。

## 今回の変更

- `mise run dev` は Compose の MariaDB を起動し、ヘルスチェックが通ってから API と Web を起動します。以前は MariaDB が停止していると API が接続タイムアウトで終了し、Vite が `/api` を転送できませんでした。既に稼働している MariaDB を使う場合は、`.env` に `DATABASE_URL` を設定して `mise run dev --no-compose-db` を使います。[mise.toml](../mise.toml)
- `stampbomb` は、指定されたリンクを `plainText` から読み取れないとき、traQ イベントの元の `text` から同じ引数位置のリンクを取得します。traQ は埋め込み表示する URL を `plainText` で `[引用メッセージ]` に置き換えます。通常の URL と traQ の引用形式を扱い、後続のリンクを対象に取り違えないことをテストしています。[stampbomb の実装](../apps/api/src/bot/commands/stampbomb.rs) · [traQ の本文変換](https://github.com/traPtitech/traQ/blob/master/utils/message/parser.go)
- Bot の全コマンドを、模擬 traQ の HTTP API と組み合わせてテストします。`stampbomb` は対象メッセージとキュー投入内容、無効な入力、キュー満杯を確認します。`senryu status` / `off` / `on` の設定保存は専用 MariaDB で確認します。[コマンドのテスト](../apps/api/tests/bot/commands.rs)

## Bot 接続の確認

Bot Console でテスト用 Bot が WebSocket Mode になっていることを確認します。`.env` の `TRAQ_BOT_ID` と `TRAQ_BOT_TOKEN` には同じ Bot の ID と Access Token を入れ、`TRAQ_BOT_NAME` にはメンションに使う Bot 名を設定します。アプリが認証ヘッダーに `Bearer ` を付けます。設定後は `mise run dev` を再起動し、`curl -fsS http://127.0.0.1:5173/api/bot/status` が `{"status":"connected"}` を返すことを確認します。[Bot Console](https://bot-console.trap.jp/) · [認証ヘッダーの実装](../apps/api/src/traq/client.rs)

今回の `401 Unauthorized` は、当時の `.env` のトークンで traQ の WebSocket 接続先と通常の API に直接問い合わせても再現しました。同じエラーが出たら、Bot Console の現在の Access Token と `.env` の `TRAQ_BOT_TOKEN` を照合してください。

## 実装の入口

| 用途                     | 編集箇所                                                                                                                                                                                                                                                               |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 起動・環境変数           | [main.rs](../apps/api/src/main.rs)、[config.rs](../apps/api/src/config.rs)。起動時に MariaDB のマイグレーションを適用します。                                                                                                                                          |
| Bot の受信・イベント処理 | [bot.rs](../apps/api/src/bot.rs)。WebSocket で受信し、通常イベントを受信順に処理します。イベント型と API クライアントは `apps/api/src/traq/` にあります。投稿・履歴取得・投稿リンクの処理は [client/messages.rs](../apps/api/src/traq/client/messages.rs) にあります。 |
| コマンド追加・ヘルプ     | [commands.rs](../apps/api/src/bot/commands.rs) で振り分けとヘルプ定義を登録します。コマンド本体は `apps/api/src/bot/commands/` に1コマンドずつ置き、ガチャの抽選処理は共用します。Bot の `help` と Web の `/api/commands` は同じ定義を使います。                       |
| スタンプ送信             | [bot/stampbomb.rs](../apps/api/src/bot/stampbomb.rs)。処理は単一ワーカーに渡し、100個ずつに分けて送ります。50 ms の待機はジョブの境目にも適用します。                                                                                                                  |
| 川柳・最上川の判定と保存 | [verse/detect.rs](../apps/api/src/verse/detect.rs) が川柳と最上川を判定します。[VerseStore](../apps/api/src/verse/store.rs) が両方の設定、保存・更新・削除、一覧ページ分割を担当します。                                                                               |
| HTTP API                 | [http.rs](../apps/api/src/http.rs)。川柳一覧は `/api/senryu`、コマンド案内は `/api/commands`、DB と Bot の確認はそれぞれ `/api/health` と `/api/bot/status` です。                                                                                                     |
| Web                      | [App.tsx](../apps/web/src/App.tsx) が React Router のルートを定義します。`/senryu` は川柳一覧、`/commands` は使い方、`/` は共通ナビゲーションだけで将来のトップページ用です。                                                                                          |

Rust は `bot.rs` + `bot/` のようにモジュール名と同じファイルを入口にします。機能の流れを追える構成を優先し、DB や traQ クライアントは必要な箇所から直接呼びます。模擬 traQ を使う Bot テストは `apps/api/tests/bot/` に置き、`#[path]` で Bot の内部テストとして読み込みます。公開 HTTP API の統合テストは `apps/api/tests/api.rs` に置きます。Web の `pages/` は取得処理と画面構成、`components/` は表示部品を担当します。MUI の配色とアクセント `#0485F7` は [theme.ts](../apps/web/src/theme.ts) にあります。[HeroUI Default テーマ](https://heroui.com/en/themes) · [UXPin の UI デザインの基本原則](https://www.uxpin.com/studio/jp/blog-jp/ui-design-principles-ja/)

## stampgacha の実装引き継ぎ

入力形式は traQ-gazer の登録欄を参考にし、パターンと末尾フラグは利用者が TypeScript で使い慣れた ECMAScript の `RegExp` に合わせます。大小文字を既定で区別しない点は Bot の仕様です。 [スタンプガチャの仕様](specs.md#スタンプガチャ)

- Rust が先頭の引数を読み、通常の部分一致と正規表現を振り分けます。正規表現では、traQ から取得したスタンプ一覧とパターン・フラグを1回の `tokio::task::spawn_blocking` に渡します。QuickJS のコンテキストはタスク内に閉じ、`RegExp` を一度コンパイルし、各名前の照合前に `lastIndex` を `0` にします。抽選と返信は Rust が担当します。 [コマンドの入口](../apps/api/src/bot/commands/stampgacha.rs)
- `rquickjs` のビルドで QuickJS の C ソースをコンパイルします。Bot はブロッキングタスクの結果を待つため、スタンプ一覧の取得後、コンパイルと照合に 250 ms の期限を設けます。照合中は QuickJS の割り込みハンドラーで期限を確認します。模擬スタンプ名1万件を WSL のデバッグビルドで照合したところ、2種類の式を各5回実行して最長は約27 ms でした。実際の traQ の一覧は認証が必要で、計測には使っていません。 [照合処理](../apps/api/src/bot/commands/stampgacha/regexp.rs) · [割り込み API](https://docs.rs/rquickjs/latest/rquickjs/struct.Runtime.html#method.set_interrupt_handler)
- テスト専用の Bot コマンドテストは `apps/api/tests/bot/` に置きます。模擬 traQ で ECMAScript の構文とフラグ、区切りのスラッシュ、入力エラー、実行中の時間切れと次のコマンドへの返信を確認します。 [コマンドのテスト](../apps/api/tests/bot/commands.rs)

## markov の実装引き継ぎ

- `markov <username> [n] [--here]` の使い方は既存のコマンド定義に登録します。チャンネルと Bot との DM で同じ構文を使います。 [マルコフ生成の仕様](specs.md#マルコフ生成) · [コマンドの入口](../apps/api/src/bot/commands.rs)
- 公開チャンネル全体とチャンネル内の検索には `GET /messages` を使用し、投稿者の UUID を `from` に渡します。チャンネルの `--here` では `in` も指定します。DM の `--here` は `GET /users/{送信者ID}/messages` で履歴を取得し、対象ユーザーの UUID で絞ります。DM の返信も同じ送信者 ID のエンドポイントに送ります。 [traQ v3 OpenAPI](https://github.com/traPtitech/traQ/blob/master/docs/v3-api.yaml)
- 検索・履歴取得から生成までを単一のワーカーに渡し、形態素解析と生成は `spawn_blocking` で実行します。川柳と共用する本文抽出ではリンクの表示ラベルを残し、マルコフ生成では引用ブロックの中身を除きます。生成結果は traQ の自動埋め込みを無効にして投稿します。 [ワーカー](../apps/api/src/bot/markov.rs) · [共用の辞書](../apps/api/src/text.rs) · [共用の本文抽出](../apps/api/src/text/markdown.rs)
- 模擬 traQ のテストで、大文字・小文字を区別しないアカウント名の照合、生成文だけの返信、チャンネルと DM での検索範囲と返信先、ページングの300件上限、DM 内の投稿者の絞り込みを確認できます。 [markov のテスト](../apps/api/tests/bot/markov.rs)
- 設定済みの Bot トークンで traQ のユーザー照会と公開検索が HTTP 200 を返すことを確認しました。実際の DM は、テスト Bot で `DIRECT_MESSAGE_CREATED` を購読してから `markov <username>` と `markov <username> --here` を試し、履歴を取得できることと返信先を確認します。 [traQ の DM イベント配信](https://github.com/traPtitech/traQ/blob/master/service/bot/handler/ev_message_created.go#L33-L57)

## 川柳設定の最新仕様

- `@BOT_lachite senryu status` / `on` / `off` は、チャンネルにコマンドを投稿できる人なら誰でも使えます。対象は投稿したチャンネルです。旧設定の `RACHITE_OPERATOR_IDS` は廃止済みです。
- チャンネル設定の初期値はオンです。オフは新規掲載だけを止めます。掲載済みの句は、元投稿の編集で再判定し、判定から外れた場合や元投稿が削除された場合に取り下げます。
- 新規投稿からの初回検出時は traQ に通知します。編集による反映は一覧だけです。同じ元メッセージ ID の重複登録は DB の一意制約で防ぎます。

## 最上川の実装引き継ぎ

2026年11月23日に、投稿全体が 5・7 拍のときの最上川の返信とスタンプ送信を実装しました。[最上川の仕様](specs.md#最上川)

1. 投稿の本文抽出、文節分け、拍数計算を川柳と共用します。川柳は投稿の途中の句を拾い、最上川は本文全体が文節の境界で 5・7 拍に分かれるかを判定します。 [判定処理](../apps/api/src/verse/detect.rs)
2. 人の新規投稿を処理する `handle_created` で、コマンド処理後に最上川を判定します。`mogamigawa status` / `on` / `off` は投稿したチャンネルの設定を表示・変更します。既存の設定行と設定未登録のチャンネルでも最上川の初期値はオンです。 [イベント処理](../apps/api/src/bot.rs) · [コマンド定義](../apps/api/src/bot/commands/mogamigawa.rs) · [マイグレーション](../apps/api/migrations/0002_mogamigawa.sql)
3. 最上川は通知を送る前に句と検出日時を保存します。スタンプ3種が揃ったら、元投稿 ID の返信処理を一度だけ開始し、返信の後にスタンプを順番に押します。Web の `/mogamigawa` は `/api/mogamigawa` から新着20件ずつ取得します。本文と日時を持たない既存の処理記録も、元投稿リンクとして表示できます。 [最上川の送信処理](../apps/api/src/bot/mogamigawa.rs) · [マイグレーション](../apps/api/migrations/0003_mogamigawa_listing.sql)
4. 判定テストでは、長音を含む 5・7 と 5・7・5、5拍目が文節境界にならない12拍の本文、URL付き本文を確認しました。模擬 traQ と専用 MariaDB では、初期オン、設定の独立、重複イベント、スタンプの送信先・順序・個数、途中失敗を確認しました。既存の川柳設定を保持したまま移行できることも確認済みです。実際の traQ では、同じスタンプが先に押されていても追加されることと、画面での表示順を確認します。 [判定テスト](../apps/api/src/verse/detect.rs) · [模擬 API のテスト](../apps/api/tests/bot/commands.rs)

## SudachiDict への移行

1. `markdown.rs` は `pulldown-cmark` のイベントから本文を抽出し、traQ 固有の埋め込み、スタンプ、メンション、ネタバレを補足処理します。`plainText` は traQ が埋め込みの一部を置換した結果なので、判定には元の `text` を使います。 [実装](../apps/api/src/text/markdown.rs) · [traQ の本文変換](https://github.com/traPtitech/traQ/blob/master/utils/message/parser.go)
2. `dictionary.rs` は Lindera の同梱 SudachiDict から読みと品詞を項目名で取得します。英字は小文字で辞書を引き、掲載には元の綴りを使います。`detect.rs` は読み不明の語で川柳の探索を区切り、最上川の判定を見送ります。 [辞書との接続](../apps/api/src/verse/dictionary.rs) · [文節と拍数の判定](../apps/api/src/verse/detect.rs) · [Lindera の辞書形式](https://lindera.github.io/lindera/lindera-sudachidict/dictionary_format.html)
3. 既存の掲載結果は移行時に再計算しません。編集時に新しい解析を使って一覧だけ更新し、削除時は従来どおり取り下げます。解析エラーと解析が成功した結果の未検出を区別します。 [イベント処理](../apps/api/src/bot.rs)
4. テストでは本文抽出と句の判定をそれぞれ確認します。同じ句を通常文・リスト・引用・リンクで書いた場合、装飾しかない投稿、読み不明の語の前後に句がある場合を含めます。模擬 traQ では両方の本文フィールドが異なるイベントを使います。 [検出仕様](specs.md#川柳と最上川の本文抽出拍数)
5. 同梱辞書は「月は東に日は西に」を一語として解析します。「菜の花や月は東に日は西に」は中七の末尾に文節境界が取れず、句ごとに空白を入れた「菜の花や 月は東に 日は西に」は検出できます。 [判定テスト](../apps/api/src/verse/detect.rs)
6. 一時 MariaDB 11.8 と模擬 traQ で `cargo test --workspace --locked -- --include-ignored` の30件を確認しました。実際の traQ では、番号付きリスト、引用文、添付を含む新規投稿と編集イベントをテスト Bot で確認します。 [イベントのテスト](../apps/api/tests/bot/commands.rs)

## 検証済みの範囲

- WSL で停止中の MariaDB から `mise run dev` を起動し、Vite 経由の `/api/health`、`/api/senryu`、`/api/commands`、`/swagger-ui/` が HTTP 200 を返すことを確認しました。`--no-compose-db` での起動も確認済みです。
- `mise run test` と `mise run check` は通過しました。専用の MariaDB 11.8 を起動し、`TEST_DATABASE_URL` を設定して `cargo test --workspace --locked -- --include-ignored` を実行したところ、DB を使う2件を含む18件が通過しました。GitHub Actions の API ジョブも同じ Cargo コマンドを実行します。[CI 設定](../.github/workflows/ci.yml)
- 模擬 traQ の WebSocket から `stampbomb w 1234` の投稿イベントを送り、引用先のメッセージに対して合計1,234回分が13リクエストで送信されることを確認しました。実際の traQ では、ユーザー提供の画面で `senryu status` への応答を確認できています。
- 前回は Web の本番ビルドと、テストデータを使った `/`、`/senryu`、`/commands` のブラウザ表示を確認しました。

## 次に確認すること

1. テスト Bot で `MESSAGE_CREATED`、`MENTION_MESSAGE_CREATED`、`MESSAGE_UPDATED`、`MESSAGE_DELETED` を購読します。未参加チャンネルからの `join`、引用リンクを使った `stampbomb` の少数回送信、一般ユーザーによる `senryu off` / `on`、投稿・編集・削除の Web 一覧への反映を実際の traQ で確認します。[メッセージイベント](https://github.com/traPtitech/traQ-bot-console/blob/dev/src/docs/bot/events/message.md) · [traQ v3 OpenAPI](https://github.com/traPtitech/traQ/blob/master/docs/v3-api.yaml)
2. 公開前に Web と `/api` を同じ Hard 認証の配下に置き、未認証で両方にアクセスできないことと、Axum に直接アクセスできないことを確認します。テスト Bot での確認後に、旧 Bot を停止して新 Bot を1インスタンスで起動します。

旧 Bot の互換性を確認する際は、隣の `../test2` にある `src/usecase/bot_service.rs`、`src/usecase/commands/`、`tests/test.rs` を参照してください。停止中のイベントとの照合、追加コマンド、LLM の用途は別の作業です。

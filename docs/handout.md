# rachite 開発引き継ぎ

仕様の基準は [specs.md](specs.md) です。現時点の合意は設計までです。以下は実装に着手するときの参照先と進め方です。

## リポジトリと参照先

- `rachite` は React + MUI／Axum のフルスタックテンプレートです。`apps/web` と `apps/api` を使い、Bot のコードは `apps/api/src/bot/` に置きます。Bot と Web API で使う川柳の処理は `apps/api/src/senryu/` にまとめる想定です。
- テンプレートは SQLite、メールアドレス・パスワード認証、サンプルの `items` CRUD を備えています。これらは MariaDB、公開経路の Hard 認証、川柳一覧とコマンド案内に置き換えます。Vite／nginx の `/api` プロキシと Axum の OpenAPI 基盤は再利用できます。現状の詳細は [TEMPLATE_REQUIREMENTS.md](TEMPLATE_REQUIREMENTS.md) にあります。
- 旧 Bot は同じ親ディレクトリの `../test2` です。コマンドの入口は `src/usecase/bot_service.rs`、コマンド本体は `src/usecase/commands/`、川柳判定は `src/domain/senryu.rs`、イベント処理は `src/handler/handler.rs`、スタンプ送信は `src/infrastructure/worker.rs` にあります。各モジュールのテストと `tests/test.rs` が互換性の確認に使えます。
- Bot 登録・イベント購読は [Bot Console の設定資料](https://github.com/traPtitech/traQ-bot-console/blob/dev/src/docs/bot/settings.md) と [イベント定義](https://github.com/traPtitech/traQ-bot-console/blob/dev/src/botEvents.ts)、イベントの JSON は [メッセージイベント資料](https://github.com/traPtitech/traQ-bot-console/blob/dev/src/docs/bot/events/message.md) を参照してください。送信 API は [traQ v3 OpenAPI](https://github.com/traPtitech/traQ/blob/master/docs/v3-api.yaml) にあります。

## 実装順

1. **旧 Bot の移植。** コマンドの振り分けは小さな `match` を起点にし、機能ごとに必要な分だけ分けます。WebSocket の受信と HTTP 返信の組み合わせはそのまま使い、通常コマンドは受信順に処理します。外部 HTTP にタイムアウトを設け、長い `stampbomb` は単一ワーカーに渡します。旧版の50 ms待ちは一つのジョブの分割送信間だけなので、移植時はジョブの境目でも適用します。
2. **MariaDB と川柳の連携。** SQLx の `mysql` ドライバと `MySqlPool`、MariaDB 用のマイグレーションを導入します。`DATABASE_URL`、`.env.example`、`compose.yaml`、`Dockerfile`、`.github/workflows/ci.yml`、SQLite を使っている `apps/api/tests/api.rs` も確認します。DB 統合テストは MariaDB で実行します。設定行がない場合は検出オンで、DB 読み取りエラーは通常のエラーとして扱います。
3. **Web の一覧とヘルプ。** 抽出した句を返す読み取り API と、一度に一定件数を表示する MUI 一覧を作ります。コマンド案内は Bot のヘルプと共通の定義を使います。画面と `/api` を Hard 認証の同じ公開経路に置き、API に直接アクセスできる別の公開経路がないことを確認します。
4. **テスト用 Bot で確認してから本番を切り替え。** Bot Console では通常投稿のために `MESSAGE_CREATED`、未参加チャンネルでの呼び出しのために `MENTION_MESSAGE_CREATED`、掲載結果の追従のために `MESSAGE_UPDATED` と `MESSAGE_DELETED` の購読を確認します。メンションは `MESSAGE_CREATED` として配信されます。未参加チャンネルからの `join` はテスト用 Bot で実際に確認します。本番では旧 Bot を停止してから新 Bot を起動します。

## 完了確認

- メンションへの応答と旧コマンドが、旧版と互換の名前・引数で動きます。ガチャは旧版と同じ条件で抽選します。
- `stampbomb` の複数ジョブを連続して投入しても、送信間隔が維持されます。キュー満杯と送信タイムアウトの扱いも確認します。
- 運用者だけが、コマンドを実行したチャンネルの川柳検出設定を変更できます。設定未登録、明示的なオン、明示的なオフを確認します。
- 投稿で一覧に1件追加され、同じ元メッセージ ID で重複しません。編集での更新、初めて検出された句の追加、検出できなくなった句と削除された投稿の取り下げを確認します。検出オフの間も掲載済みの句への編集・削除は反映されます。
- 未認証のアクセスは Web と一覧 API の両方で拒否されます。認証済みの部員は一覧とコマンドの使い方を閲覧できます。
- デプロイ先で Web にアクセスがない間も Bot が稼働します。Web API のヘルスチェックと Bot の WebSocket 接続状況は別々に確認できるようにします。

川柳判定の作り直し、停止中の変更との照合、追加コマンド、LLM の用途はそれぞれ別の作業として扱います。初版では旧 Bot の互換動作と、川柳の掲載・更新・取り下げを優先します。

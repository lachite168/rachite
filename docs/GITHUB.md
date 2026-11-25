# GitHub の設定ガイド

| 設定ファイル                                           | 役割                      |
| ------------------------------------------------------ | ------------------------- |
| [CI ワークフロー](../.github/workflows/ci.yml)         | Web と API の検査・ビルド |
| [Dependabot](../.github/dependabot.yml)                | 依存関係の更新 PR         |
| [PR テンプレート](../.github/pull_request_template.md) | 変更理由と検証結果の記録  |

## GitHub Actions と CI

`CI` は PR の作成・再オープン・追加 push と、`main` への push で動きます。`Web` と `API` は `ubuntu-latest` 上で並行に実行します。同じ PR に追加 push があれば古い実行を打ち切ります。

| ジョブ | 実行内容                                                                                                                                                     |
| ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Web    | `.node-version` の Node.js で `npm ci`、Oxlint、Prettier、Web テスト、型検査と Vite の本番ビルド                                                             |
| API    | Rust stable で rustfmt、Clippy、MariaDB 11.8 を使うテスト、リリースビルド。テストでは CI の `TEST_DATABASE_URL` を使い、通常スキップする DB 依存テストも実行 |

結果は PR の **Checks** またはリポジトリの **Actions → CI** で確認できます。手元で同等の検査をする方法は [開発・保守ガイドの検証手順](handout.md#検証) を参照してください。

ワークフローは読み取り専用のリポジトリ権限を使い、checkout 後に認証情報を保持しません。外部アクションはコミット SHA で固定し、対応するバージョンを行末コメントに記載します。 [GitHub Actions の構文](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax)・[GitHub Actions の安全な使い方](https://docs.github.com/en/actions/reference/security/secure-use)

CI の実行範囲はビルドまでです。公開は [NeoShowcase の手順](NEOSHOWCASE.md) に従います。CI の成功をマージ条件にする場合は、GitHub のルールセットまたはブランチ保護で `Web` と `API` を必須にします。 [GitHub のブランチ保護](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches)

## Dependabot とバージョン更新

- デフォルトブランチの設定で、npm、Cargo、GitHub Actions の更新候補を毎月調べます。更新 PR は CI の結果を確認してからマージします。
- npm は公開後7日間の待機期間を設け、型定義、Oxlint、Prettier、TypeScript を `tooling` グループにまとめます。手元の npm でも `.npmrc` の `min-release-age=7` を使います。
- GitHub Actions はパッチ・マイナー更新をまとめ、メジャー更新は個別に確認します。
- Node.js 本体を更新するときは `.node-version`、Web 用 `Dockerfile` のイメージ、`package.json` の `engines` を揃えます。`npm ci` と CI 相当の検査・ビルドを実行し、依存の解決結果は `package-lock.json` と `Cargo.lock` で管理します。

[Dependabot 設定リファレンス](https://docs.github.com/en/code-security/reference/supply-chain-security/dependabot-options-reference)

## PR テンプレート

`What` に変更内容、`Why` に理由、`Verification` に実行した検証、`References` に関連資料を記載します。

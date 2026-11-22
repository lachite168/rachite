# GitHub の設定ガイド

このリポジトリの GitHub 設定は、主に下の3ファイルで管理しています。設定ファイルをコミットして GitHub に反映すると、GitHub が読み取ります。

| ファイル                                                                  | 役割                                             |
| ------------------------------------------------------------------------- | ------------------------------------------------ |
| [`.github/workflows/ci.yml`](../.github/workflows/ci.yml)                 | コード変更時にチェックを実行する                 |
| [`.github/dependabot.yml`](../.github/dependabot.yml)                     | 新しい依存バージョンを探し、更新を PR で提案する |
| [`.github/pull_request_template.md`](../.github/pull_request_template.md) | PR の説明欄にひな形を入れる                      |

## GitHub Actions と CI

GitHub Actions は自動処理を実行する GitHub の機能です。ワークフローは、処理を始める条件と手順を記述したファイルです。このプロジェクトのワークフロー名は `CI` です。ここでの CI は、変更したコードを自動で検査して、本番向けのビルドまで成功するか確かめることを指します。

- **いつ動くか**。PR を作成・再オープンしたときや、PR を開いた状態でコミットを push したときに動きます。メインブランチ `main` に変更が入ったときにも動きます。PR を開いていない作業ブランチへの push は、現在の設定では対象外です。
- **どこで動くか**。GitHub が用意する `ubuntu-latest` の実行環境で動きます。`Web` と `API` は別のジョブなので並行して進み、失敗を片方ずつ確認できます。各ジョブには実行環境と手順の一覧があり、`uses` は既存アクションの呼び出し、`run` はコマンド実行を指します。
- **重複時の動作**。同じ PR に追加で push すると、その PR の古い実行を打ち切り、新しいコミットを検査します。`main` の push は実行ごとに分けています。

| ジョブ | 実行すること                                                                                                                                                 |
| ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Web    | `.node-version` の版で Node.js をセットアップし、`npm ci` で lockfile どおりに依存を導入。Oxlint、Prettier、TypeScript の型検査、Vite の本番ビルドを実行     |
| API    | Rust stable に rustfmt と Clippy を追加し、Rust の整形、lint、SQLite を使う API 統合テスト、リリースビルドを実行。Cargo の検査・ビルドでは `--locked` を使う |

両ジョブは最初に checkout でコードを取得します。npm と Rust のキャッシュは、再取得・再ビルドを減らすためのものです。Node.js の版はローカルの mise と CI が同じファイルから読み、Docker の Web 用イメージもその版に揃えています。

`permissions: contents: read` は CI 用トークンをリポジトリ内容の読み取りに限定する設定です。checkout の `persist-credentials: false` は、そのトークンを後続の Git コマンドで使える状態で残さない設定です。呼び出すアクションは `@v7` のような可動タグではなく、40桁のコミット SHA で固定しています。どの版の SHA かは行末のコメントで確認できます。

GitHub では、リポジトリの **Actions → CI** から実行を選び、**Web** または **API** を開くとステップごとのログが見られます。PR の **Checks** タブにも結果が出ます。たとえば `Type-check and build` が失敗したら、そのログに型エラーのファイル名やビルドエラーが出ます。同じ主要チェックは手元でも `mise run check` と `mise run build` で実行できます。

このワークフローの役割はビルドまでです。Docker イメージの公開や本番デプロイを行う CD は設定していません。また、CI に失敗すると PR に結果が表示されますが、マージを止めるには GitHub のルールセットやブランチ保護で `Web` と `API` のチェックを必須にする必要があります。このプロジェクトで用意したのは設定ファイルで、GitHub 上のルールセットやブランチ保護は変更していません。

[GitHub Actions の構文](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax)・[GitHub Actions の安全な使い方](https://docs.github.com/en/actions/reference/security/secure-use)・[ブランチ保護](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches)

## Dependabot とバージョン更新

Dependabot はデフォルトブランチの設定を読み、月に一度 npm、Cargo、GitHub Actions の更新候補を調べます。候補があれば変更内容を PR で提案します。PR を確認し、CI 結果を見てマージします。自動マージは設定していません。

- npm は公開直後の更新を避けるため7日間空けます。型定義、Oxlint、Prettier、TypeScript は `tooling` グループでまとめます。手元の npm にも `.npmrc` の `min-release-age=7` を設けています。
- Cargo は Rust のクレート、GitHub Actions はワークフロー内のアクションが対象です。Actions のパッチ・マイナー更新はまとめ、メジャー更新は個別に確認します。
- Node.js 本体の `.node-version` と Dockerfile の Node.js イメージはこの設定の対象外です。Node.js を更新するときは両方と、`package.json` の `engines` を揃え、`npm ci`・CI 相当の検査・ビルドで確認します。アプリの依存は `package-lock.json` と `Cargo.lock` に記録します。

[Dependabot 設定リファレンス](https://docs.github.com/en/code-security/reference/supply-chain-security/dependabot-options-reference)

## PR テンプレート

新規 PR の説明欄に `What`、`Why`、`Verification`、`References` が入ります。たとえば Node.js を更新するなら、変更した版、更新理由、実行したビルドやテスト、リリースノートの順で書くと、後から変更の判断を追えます。

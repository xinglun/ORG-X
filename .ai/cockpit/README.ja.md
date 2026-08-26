# インストール済み AI Cockpit Runtime

ORG-X は、外部にインストールされた Rust `ai-cockpit` Runtime で管理します。
このディレクトリには、旧 Python/Make Runtime は含まれません。

## Repository-bound コマンド

すべてのコマンドで、この repository を明示的に指定します。

```bash
ai-cockpit inspect --repo <repo>
ai-cockpit status --repo <repo>
ai-cockpit compatibility --repo <repo>
ai-cockpit doctor --repo <repo>
ai-cockpit agent doctor --repo <repo> --json
```

新しい checkout では、repository 所有の protocol を attach して calibrate します。

```bash
ai-cockpit attach --repo <repo>
ai-cockpit profile confirm --repo <repo> --program cargo --args test,--workspace
```

Agent adapter の install は明示的かつ repository-local であり、home directory の
設定は変更しません。

```bash
ai-cockpit agent list --repo <repo>
ai-cockpit agent install --repo <repo> --provider codex
```

## Work Item lifecycle

```bash
ai-cockpit work-item new --repo <repo> --id <id> --mode code
ai-cockpit start --repo <repo> --id <id> --intent "..." --goal "..." \
  --scope 'src/**' --authority authorized
ai-cockpit preflight --repo <repo> \
  --contract .ai/work-items/active/<id>.contract.json
ai-cockpit checkpoint --repo <repo> --id <id>
ai-cockpit verify --repo <repo> --work-item <id> \
  --command cargo --args test,--workspace --workers 1
ai-cockpit finish --repo <repo> --id <id>
ai-cockpit work-item outcome --repo <repo> --id <id>
ai-cockpit archive --repo <repo> --id <id>
```

Preflight は evidence から判定されます。`not_ready` と
`needs_human_confirmation` は必ず停止し、yellow は declared verification が
safe action の場合だけ進めます。`finish` には current verification evidence、
green に更新された decision、ちょうど一つの checkpoint が必要です。archive 前に
human Outcome を会話へ提示し、close は reviewed PR の merge と default branch の
同期後だけ実行します。

`.ai/work-items/archive/`、recovery receipt、install evidence の旧 V1 記録は
immutable な履歴として保持します。これらは現在の Runtime state ではありません。

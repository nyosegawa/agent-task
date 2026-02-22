# task

Coding Agent と人間が非同期で協調するための軽量タスク管理CLI。

テキストファイル (`~/.local/share/tasks/tasks.log`) を single source of truth とし、MCP や外部サービスに依存しない。

## Install

```bash
cargo install --git ssh://git@github.com/nyosegawa/agent-task.git
```

## Commands

```bash
task write <status> <note>       # タスク作成
task doing <id>                  # セッション開始宣言
task reviewing <id> <pr_url>     # PR作成・レビュー依頼
task get [--status <status>]     # タスク一覧取得
```

### Status lifecycle

```
inbox → todo → doing → blocked → inreview → done
```

| status   | 操作者    | 説明                     |
|----------|----------|--------------------------|
| inbox    | 人間のみ  | 未分類・未判断            |
| todo     | 人間のみ  | やると決まった、未着手     |
| doing    | Agent    | 作業中                   |
| blocked  | Agent    | 人間の介入が必要          |
| inreview | Agent    | PR作成済み、レビュー待ち   |
| done     | 人間のみ  | 完了                     |

## stdout output

stdout出力がセッションログとの紐づけに使われる。

```
task write todo "認証機能を実装"    → TASK_ADD_a3f8c2d1
task doing a3f8c2d1               → TASK_DOING_a3f8c2d1
task reviewing a3f8c2d1 <pr_url>  → TASK_REVIEWING_a3f8c2d1
```

## Storage format

```
{ID} | {project} | {status} | {title} | {description/url}
```

- ID: 8文字ランダムhex
- project: `git remote get-url origin` から `owner/repo` 形式を抽出。git管理外はcwdフルパス
- append-only: edit/delete禁止。各IDの最新エントリが現在状態

## Agent workflow

```
1. セッション開始  → task doing <id>
2. 作業中に詰まる  → task write blocked "理由"
3. plan review時   → task write blocked "plan review: {サマリー}"
4. 作業完了        → gh pr create ... && task reviewing <id> <pr_url>
```

## Session log との連携

stdoutに出力されたプレフィックス付きIDがsession logのjsonlに自動記録される。

```bash
# タスクa3f8c2d1に関わった全セッションを特定
grep -r "TASK_DOING_a3f8c2d1" ~/.claude/projects/

# セッションが担当したタスク一覧
grep "TASK_" ~/.claude/projects/.../session.jsonl
```

## Development

```bash
cargo test                    # ユニット + インテグレーション
cargo fmt --check
cargo clippy -- -D warnings
```

## License

MIT

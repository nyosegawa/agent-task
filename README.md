# task

Coding Agent と人間が非同期で協調するための軽量タスク管理CLI。

JSONL ファイル (`~/.local/share/tasks/tasks.log`) を single source of truth とし、MCP や外部サービスに依存しない。

## Why

複数の Coding Agent を並行運用すると「いま何が動いていて、何が詰まっているか」の把握が急激に難しくなる。既存のタスク管理（GitHub Issues, Linear, Jira 等）は人間同士の協調には最適化されているが、Agent との非同期協調には重すぎる。MCP 経由で外部サービスに繋ぐアプローチもあるが、Agent ごとに MCP の設定・認証・レイテンシが発生し、Agent の数だけオーバーヘッドが増える。

このCLIは、Agent が自律的にシェルコマンドを叩けることで成立する:

- **stdout がインターフェース**: `task create` は `task created! ID: <id>`（人間向け）と `TASK_ADD_<id>`（機械向け）を出力し、`task update` は `TASK_DOING_a3f8c2d1` のようなプレフィックス付きIDを出力する。セッションログとタスクを紐づけできる。MCP不要、API不要
- **append-only JSONL ログ**: `tasks.log` は追記のみ。edit/delete 禁止。各IDの最新エントリが現在状態。ロック不要で複数 Agent の同時書き込みに耐える
- **Agent 非依存**: Claude Code, Codex, Gemini CLI, Cursor, Cline, OpenCode, Antigravity — どの Agent でも instruction ファイルに数行追記するだけで導入できる

## Install

```bash
cargo install agent-task
```

## Commands

```bash
task create "<title>" ["<description>"] [--status <status>]    # タスク作成（デフォルト: todo）
task update <id> <status> ["<note>"] [--description "<desc>"]  # ステータス更新（ID存在チェックあり）
task list [<status>] [--all]                                   # 一覧（デフォルト: 現プロジェクト、--all: 全プロジェクト）
task get <id>                                                  # 詳細・状態遷移履歴
task init [--global]                                           # instruction snippet を Agent 設定ファイルに注入
```

### Status

CLI は status を制限しない。任意の文字列を `task create --status` / `task update` / `task list` で使える。

以下は規約として定義している status:

| status   | 説明                     |
|----------|--------------------------|
| inbox    | 未分類・未判断            |
| todo     | やると決まった、未着手     |
| doing    | 作業中                   |
| blocked  | 人間の介入が必要          |
| inreview | PR作成済み、レビュー待ち   |
| done     | 完了                     |

Agent が instruction ファイル経由で認識・操作する status は以下の範囲:

```
todo → doing → blocked → inreview / done
```

タスク作成（`task create`）は人間・Agent 双方が行える。Agent が sub-task を切る、blocked で起票するなどのケースがある。

### task init

プロジェクト内の既存 instruction ファイルを検出し、instruction snippet を追記（改行2つ + snippet）する。

```bash
task init           # プロジェクトルートの既存ファイルに注入
task init --global  # グローバル設定ファイルに注入
```

**ローカル（`task init`）**: プロジェクトルートに以下のファイルが存在すれば注入:

| 検出対象 | 注入形式 |
|---------|---------|
| `CLAUDE.md` | `## Override Rule: Task Management` + snippet 追記 |
| `AGENTS.md` | `## Override Rule: Task Management` + snippet 追記 |
| `GEMINI.md` | `## Override Rule: Task Management` + snippet 追記 |
| `.cursor/rules/` | `task-management.mdc` を作成（frontmatter付き） |
| `.clinerules/` | `task-management.md` を作成（`# Override Rule: Task Management` + snippet） |

**グローバル（`task init --global`）**: Agent の設定ディレクトリが存在すれば注入:

| 検出対象 | 注入先 |
|---------|-------|
| `~/.claude/` | `~/.claude/CLAUDE.md` |
| `~/.codex/` | `~/.codex/AGENTS.md` |
| `~/.gemini/` | `~/.gemini/GEMINI.md` |
| `~/.config/cline/` | `~/.config/cline/rules/task-management.md` |
| `~/.config/opencode/` | `~/.config/opencode/AGENTS.md` |

- 既に snippet が含まれている場合はスキップ（冪等）
- 注入したファイル一覧を stdout に表示

### task list のスコープ

`task list` はデフォルトで cwd のプロジェクト（`git remote get-url origin` から判定）に絞り込む。`--all` で全プロジェクト横断表示。

### task get の出力

`task get <id>` はそのIDの全ログエントリ（状態遷移履歴）を時系列で表示する:

```
a3f8c2d1 | nyosegawa/agent-task | 認証機能を実装
  OAuth2で認証フローを実装

  2026-02-22T14:30:00+09:00  todo
  2026-02-22T15:00:00+09:00  doing
  2026-02-22T16:20:00+09:00  blocked    外部API仕様が未確定
  2026-02-22T17:00:00+09:00  inreview   https://github.com/.../pull/42
```

description はヘッダ下に表示。note は各遷移の右に表示。複数行はインデント。

## stdout output

stdout出力がセッションログとの紐づけに使われる。`task create` は2行出力で、2行目の `TASK_ADD_{id}` を機械処理に使う（`TASK_CREATED_{id}` は `task update <id> created` と衝突するため使わない）。

```
task create "認証機能を実装"                                       → task created! ID: a3f8c2d1
                                                                  → TASK_ADD_a3f8c2d1
task create "DB移行スクリプト" "PostgreSQL 15対応"                  → task created! ID: b7e1d4f2
                                                                  → TASK_ADD_b7e1d4f2
task create "あとで考える" --status inbox                           → task created! ID: c9d3e5a0
                                                                  → TASK_ADD_c9d3e5a0
task update a3f8c2d1 doing                                         → TASK_DOING_a3f8c2d1
task update a3f8c2d1 blocked "外部API仕様が未確定"                 → TASK_BLOCKED_a3f8c2d1
task update a3f8c2d1 blocked "仕様未確定" --description "OAuth2+OIDC" → TASK_BLOCKED_a3f8c2d1
task update a3f8c2d1 inreview "https://github.com/.../pull/42"     → TASK_INREVIEW_a3f8c2d1
task update a3f8c2d1 done                                          → TASK_DONE_a3f8c2d1
task list                                                          → (テーブル形式、プレフィックスなし)
task get a3f8c2d1                                                  → (遷移履歴、プレフィックスなし)
```

## Storage format

JSONL（1行1JSONオブジェクト）。ファイル: `~/.local/share/tasks/tasks.log`

```jsonl
{"ts":"2026-02-22T14:30:00+09:00","id":"a3f8c2d1","project":"nyosegawa/agent-task","status":"todo","title":"認証機能を実装","description":"OAuth2で認証フローを実装","note":""}
{"ts":"2026-02-22T15:00:00+09:00","id":"a3f8c2d1","project":"nyosegawa/agent-task","status":"doing","title":"認証機能を実装","description":"OAuth2で認証フローを実装","note":""}
{"ts":"2026-02-22T16:20:00+09:00","id":"a3f8c2d1","project":"nyosegawa/agent-task","status":"blocked","title":"認証機能を実装","description":"OAuth2で認証フローを実装","note":"外部API仕様が\n未確定"}
{"ts":"2026-02-22T17:00:00+09:00","id":"a3f8c2d1","project":"nyosegawa/agent-task","status":"inreview","title":"認証機能を実装","description":"OAuth2で認証フローを実装","note":"https://github.com/.../pull/42"}
```

| フィールド | 説明 |
|-----------|------|
| `ts` | ISO 8601（ローカルタイムゾーン付き）。CLI が自動付与 |
| `id` | 8文字ランダムhex |
| `project` | `git remote get-url origin` → `owner/repo` 形式。git管理外はcwdフルパス |
| `status` | 任意の文字列 |
| `title` | タスク名 |
| `description` | タスクの説明。`create` 時に設定、`update --description` で更新可能、それ以外は自動引き継ぎ。複数行対応 |
| `note` | 遷移ごとのコンテキスト。blocked の理由、PR URL 等 |

- append-only: edit/delete禁止。各IDの最新エントリが現在状態
- `task update` 時、`title` と `description` は前回エントリから自動引き継ぎ（`--description` 指定時は上書き）
- grep でタスク検索可能: `grep "a3f8c2d1" tasks.log`
- jq で構造的に処理可能: `jq 'select(.status=="blocked")' tasks.log`

## Agent workflow

```
── 開始 ──
1. タスク選択    → task list doing（残タスク確認）→ task list todo → task create（なければ新規）
2. 着手         → task update <id> doing

── 作業中 ──
3. 詰まる       → task update <id> blocked "理由"
4. plan review  → task update <id> blocked "plan review: {サマリー}"

── 終了（いずれか1つ）──
5a. PR作成      → gh pr create ... && task update <id> inreview <pr_url>
5b. 直接commit  → task update <id> done
5c. 不要        → task update <id> done "不要"
5d. 終われない  → task update <id> blocked "理由"
```

## Coding Agent 別セットアップ

```bash
task init           # プロジェクトルートの既存 instruction ファイルに snippet を注入
task init --global  # グローバル設定ファイルに snippet を注入
```

`task init` は既存ファイルを検出し、改行2つ + snippet を追記する。詳細は [task init](#task-init) を参照。

### injection snippet

`task init` が注入する内容:

```
## Override Rule: Task Management
REQUIRED: manage tasks with `task` CLI before, during, and after work. Never skip.
Commands: `task list [status]` | `task create "<title>" ["<desc>"]` | `task update <id> <status> ["<note>"]` | `task get <id>`
Session start: `task list doing` (resume) → `task list todo` (pick) → `task create` (if none). Always `task update <id> doing` before work.
During: human input needed → `task update <id> blocked "<reason>"`. Resumed → `task update <id> doing`.
End: PR → `task update <id> inreview "<pr_url>"`. Direct commit → `task update <id> done`. Unfinished → `blocked`. Unnecessary → `done`.
```

### Agent 別リファレンス

| Agent | Instruction file | Global path | Session log |
|-------|-----------------|-------------|-------------|
| Claude Code | `CLAUDE.md` | `~/.claude/CLAUDE.md` | `~/.claude/projects/<path>/<session>.jsonl` |
| Codex CLI | `AGENTS.md` | `~/.codex/AGENTS.md` | `~/.codex/sessions/YYYY/MM/DD/<name>.jsonl` |
| Gemini CLI | `GEMINI.md` | `~/.gemini/GEMINI.md` | `~/.gemini/tmp/<hash>/chats/` |
| Antigravity | `GEMINI.md` | `~/.gemini/GEMINI.md` | `~/.gemini/antigravity/conversations/` |
| Cursor | `.cursor/rules/*.mdc` | — | — |
| Cline | `.clinerules/*.md` | `~/.config/cline/rules/` | `~/.cline/log/` |
| OpenCode | `AGENTS.md` | `~/.config/opencode/AGENTS.md` | — |

- `AGENTS.md` に書けば Codex / OpenCode / Cursor / Cline の4つをカバーできる
- Cursor は `AGENTS.md` もフォールバックで読む。Cline も同様
- 最小構成は **`CLAUDE.md` + `AGENTS.md` + `GEMINI.md`** の3ファイル

## Session log との連携

stdoutに出力されたプレフィックス付きIDがsession logのjsonlに自動記録される。

```bash
# タスクに関わった全セッションを横断検索
grep -r "TASK_DOING_a3f8c2d1" ~/.claude/projects/ ~/.codex/sessions/ ~/.gemini/tmp/ ~/.cline/log/

# セッションが担当したタスク一覧
grep "TASK_" ~/.claude/projects/.../session.jsonl
```

## GUI との連携

GUI アプリケーションを構築する場合、`tasks.log` を読み書きするだけで実装できる。

### アーキテクチャ

```
tasks.log ← task CLI（Agent がシェルで実行）
          ← GUI（人間が操作）

GUI → tasks.log を読んで表示するだけ
```

GUI は `tasks.log` の consumer であり producer でもある。Agent と GUI が同じファイルに append する。

### タスクカード表示

セッションログと突き合わせることで、タスクカードにリッチな情報を表示できる。

| stdout イベント | 表示内容 |
|----------------|---------|
| `TASK_ADD_{id}` | 起票元（セッションログに存在すれば Agent 起票、なければ人間/GUI 起票） |
| `TASK_DOING_{id}` | 作業セッション一覧・使用 Agent・コンテキスト使用量 |
| `TASK_BLOCKED_{id}` | ブロック理由（note）・介入待ち時間 |
| `TASK_INREVIEW_{id}` | PR URL（note）・レビュー依頼セッション |
| `TASK_DONE_{id}` | 完了タイムスタンプ・総所要時間 |

### セッションログの場所

各 Agent の session log パスは [Agent 別リファレンス](#agent-別リファレンス) を参照。

### 起票者の判定

追加メタデータ不要。セッションログの有無が証跡になる:

- セッションログに `TASK_ADD_{id}` がある → **Agent 起票**
- セッションログにない → **人間起票**（GUI 経由）

### セッションログからの状態推定

instruction で `task update blocked` を指示しても、Agent は plan mode やユーザーへの質問で人間の入力待ちになった際に `blocked` への更新をスキップすることが多い。tasks.log 上は `doing` のまま実際には止まっている状態が発生する。

これを補うため、GUI はセッションログを読み取り、タスクの実質的な状態を推定する。blocked の原因は plan mode に限らず、ユーザーへの質問・外部入力待ち・セッション断絶など多岐にわたる。

#### 人間の入力待ち検出

| Agent | シグナル | 意味 |
|-------|---------|------|
| Claude Code | `tool_result` に `"Entered plan mode"` | plan review 待ち |
| Claude Code | `tool_result` に `AskUserQuestion` の結果 | ユーザーへの質問待ち |
| Codex | `turn_context` の `collaboration_mode.mode = "plan"` | plan review 待ち |
| Codex | `event_msg` の `type = "task_complete"` 後に応答なし | ユーザー入力待ち |

#### 推定ルール

| session log のシグナル | 推定状態 |
|---|---|
| `TASK_DOING_{id}` が最新 + セッションがアクティブ + Agent が出力中 | doing（作業中）|
| `TASK_DOING_{id}` が最新 + 人間の入力待ちシグナル検出 | blocked（人間の入力待ち）|
| `TASK_DOING_{id}` が最新 + セッション終了・後続の更新なし | blocked（セッション切れ）|

tasks.log 上のステータスと矛盾する場合は、セッションログ側の推定を優先する。

### 実装方針

- **表示**: `jq -s 'group_by(.id) | map(last)' tasks.log` で各IDの最新エントリ（= 現在状態）を取得。タイムスタンプから経過時間や時系列ビューも構築可能
- **人間操作**: GUI から `task create` / `task update` CLI を呼び出す
- **リアルタイム更新**: `tasks.log` を `inotify` / `FSEvents` で watch
- **PR 展開**: note に GitHub PR URL が含まれる場合、`gh api` で詳細を取得して展開表示
- **PR 中心ビュー**: `inreview` のタスクを note の PR URL で group_by すれば、PR 単位で紐づくタスク群を表示できる。複数タスクが1つの PR にまとまるケースに対応
- **セッション状態推定**: セッションログを watch し、plan mode 等のシグナルからタスク状態を補完

## Development

```bash
cargo test
cargo fmt --check
cargo clippy -- -D warnings
```

## License

MIT

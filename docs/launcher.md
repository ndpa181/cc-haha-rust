# Claude Code 模型切換指南

## 概述

通過獨立的啟動腳本，每個模型使用各自的 `~/.claude/settings.<name>.json`，
互不干擾。支援兩種執行方式：

| 前綴 | 執行二進位 | 位置 |
|------|-----------|------|
| `claude-*` | 項目 bun 啟動（`bin/claude-haha`） | 項目內 `bin/` |
| `cc-*` | 系統 `claude`（brew 或系統安裝） | `~/bin/` |

## 可用腳本

| 命令 | 模型 | 設定檔 |
|------|------|--------|
| `claude-mimo` / `cc-mimo` | MIMO（小米） | `~/.claude/settings.mimo.json` |
| `claude-qwen` / `cc-qwen` | 通義千問 | `~/.claude/settings.qwen.json` |
| `claude-minimax` / `cc-minimax` | MiniMax | `~/.claude/settings.minimax.json` |
| `claude-kimi` / `cc-kimi` | Kimi（月之暗面） | `~/.claude/settings.kimi.json` |
| `claude-deepseek` / `cc-deepseek` | DeepSeek | `~/.claude/settings.deepseek.json` |
| `claude-opus` / `cc-opus` | Claude Opus | `~/.claude/settings.opus.json` |
| `claude-glm` / `cc-glm` | GLM（智譜） | `~/.claude/settings.glm.json` |
| `claude-local` / `cc-local` | 預設 | `~/.claude/settings.local.json` |
| `claude-haha` / `cc-haha` | Haha | `~/.claude/settings.haha.json` |
| `cc-mini` | MIMO（本地版） | `~/.claude/settings.mimo.json` |

註：`cc-mini` 特殊——使用 `~/.local/bin/claude`（本地安裝版），
其餘 `cc-*` 使用系統 `claude`。

## 設定檔格式

`~/.claude/settings.<name>.json`：

```json
{
  "env": {
    "ANTHROPIC_AUTH_TOKEN": "sk-...",
    "ANTHROPIC_BASE_URL": "https://api.example.com/anthropic",
    "ANTHROPIC_MODEL": "model-name",
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "model-sonnet",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL": "model-flash",
    "ANTHROPIC_DEFAULT_OPUS_MODEL": "model-pro",
    "ANTHROPIC_REASONING_MODEL": "model-reasoning"
  }
}
```

## 原理

- `--settings` 是 Claude Code 內部**最高優先級**設定源
- 其 `env` 會覆蓋所有其他 `settings.json` 的同名變數
- `.env` 中的非敏感設定（如 `DISABLE_TELEMETRY`）仍然保留
- 每個腳本獨立執行，互不影響，可同時運行多個不同模型的 session

## 新增模型

只需 3 步：

1. 在 `~/src/cc-haha-rust/bin/` 複製一份 `claude-mimo`，改名並替換 `settings.*.json` 路徑
2. 在 `~/bin/` 建立對應的 `cc-*` 腳本
3. 建立對應的 `~/.claude/settings.<name>.json` 設定檔

## 安裝

在新機器上執行安裝腳本：

```bash
bash setup-claude-launchers.sh
```

安裝腳本會自動建立 `~/bin/` 目錄、所有 `cc-*` 腳本，並確保 `~/bin` 在 PATH 中。

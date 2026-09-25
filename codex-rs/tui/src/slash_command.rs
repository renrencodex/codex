use strum::IntoEnumIterator;
use strum_macros::AsRefStr;
use strum_macros::EnumIter;
use strum_macros::EnumString;
use strum_macros::IntoStaticStr;

/// Commands that can be invoked by starting a message with a leading slash.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, EnumString, EnumIter, AsRefStr, IntoStaticStr,
)]
#[strum(serialize_all = "kebab-case")]
pub enum SlashCommand {
    // DO NOT ALPHA-SORT! Enum order is presentation order in the popup, so
    // more frequently used commands should be listed first.
    Model,
    Provider,
    Ide,
    Permissions,
    Keymap,
    Vim,
    #[strum(serialize = "setup-default-sandbox")]
    ElevateSandbox,
    Experimental,
    #[strum(to_string = "approve")]
    AutoReview,
    Memories,
    Skills,
    Import,
    Hooks,
    Review,
    Rename,
    New,
    Archive,
    Delete,
    Resume,
    Fork,
    Worktree,
    App,
    Init,
    Compact,
    Recap,
    Plan,
    Voice,
    Goal,
    Agents,
    Side,
    Btw,
    Copy,
    Export,
    Raw,
    Tui,
    Diff,
    Mention,
    Status,
    Daemon,
    Warnings,
    Cd,
    #[strum(to_string = "pwd", serialize = "cwd")]
    Pwd,
    Usage,
    DebugConfig,
    Title,
    Statusline,
    Theme,
    #[strum(to_string = "pets", serialize = "pet")]
    Pets,
    Mcp,
    Apps,
    Plugins,
    Logout,
    Quit,
    Exit,
    Feedback,
    Rollout,
    Ps,
    #[strum(to_string = "stop", serialize = "clean")]
    Stop,
    Clear,
    TestApproval,
    #[strum(serialize = "subagents")]
    MultiAgents,
    // Debugging commands.
    #[strum(serialize = "debug-m-drop")]
    MemoryDrop,
    #[strum(serialize = "debug-m-update")]
    MemoryUpdate,
}

impl SlashCommand {
    /// User-visible description shown in the popup.
    pub fn description(self) -> &'static str {
        match self {
            SlashCommand::Feedback => "向维护者发送日志",
            SlashCommand::New => "在当前对话中开始新会话",
            SlashCommand::Init => "创建包含 Codex 项目说明的 AGENTS.md 文件",
            SlashCommand::Compact => "总结对话以避免达到上下文上限",
            SlashCommand::Recap => "立即总结当前对话",
            SlashCommand::Review => "审查当前更改并查找问题",
            SlashCommand::Rename => "重命名当前会话",
            SlashCommand::Resume => "恢复已保存的会话",
            SlashCommand::Archive => "归档当前会话",
            SlashCommand::Delete => "永久删除当前会话",
            SlashCommand::Clear => "清空终端并开始新会话",
            SlashCommand::Fork => "派生当前会话",
            SlashCommand::Worktree => "在新的工作树中开始或继续会话",
            SlashCommand::App => "在桌面应用中继续此会话",
            SlashCommand::Quit | SlashCommand::Exit => "退出 Codex",
            SlashCommand::Copy => "复制上一条回复或其中一部分",
            SlashCommand::Export => "将对话导出为 Markdown",
            SlashCommand::Raw => "切换原始回滚模式，便于在终端中选择复制",
            SlashCommand::Tui => "choose the TUI mode for the next launch",
            SlashCommand::Diff => "显示 Git 差异（包括未跟踪文件）",
            SlashCommand::Mention => "提及文件",
            SlashCommand::Skills => "使用技能帮助 Codex 更好地完成特定任务",
            SlashCommand::Import => "从 Claude Code 导入设置、当前项目和最近会话",
            SlashCommand::Hooks => "查看和管理生命周期钩子",
            SlashCommand::Daemon => "管理本地后台服务器",
            SlashCommand::Warnings => "view retained warnings and diagnostic details",
            SlashCommand::Status => "显示当前会话配置和 Token 用量",
            SlashCommand::Cd => "更改当前工作目录",
            SlashCommand::Pwd => "显示当前工作目录",
            SlashCommand::Usage => "查看账户用量或重置用量限制",
            SlashCommand::DebugConfig => "显示配置层和要求来源以便调试",
            SlashCommand::Title => "配置终端标题中显示的项目",
            SlashCommand::Statusline => "配置状态栏中显示的项目",
            SlashCommand::Theme => "选择语法高亮主题",
            SlashCommand::Pets => "选择或隐藏终端宠物",
            SlashCommand::Ps => "列出后台终端",
            SlashCommand::Stop => "停止所有后台终端",
            SlashCommand::MemoryDrop => "DO NOT USE",
            SlashCommand::MemoryUpdate => "DO NOT USE",
            SlashCommand::Model => "选择要使用的模型和推理强度",
            SlashCommand::Provider => "切换模型提供商或添加第三方提供商",
            SlashCommand::Ide => "包含 IDE 中的当前选区、打开的文件和其他上下文",
            SlashCommand::Plan => "切换到计划模式",
            SlashCommand::Voice => "启动或停止语音；使用 /voice settings 选择声音",
            SlashCommand::Goal => "设置或查看长时间运行任务的目标",
            SlashCommand::Agents => "打开智能体指挥中心",
            SlashCommand::MultiAgents => "在当前会话的子智能体之间切换",
            SlashCommand::Side | SlashCommand::Btw => "在临时派生会话中开始旁路对话",
            SlashCommand::Permissions => "选择允许 Codex 执行的操作",
            SlashCommand::Keymap => "重新映射 TUI 快捷键",
            SlashCommand::Vim => "切换输入框的 Vim 模式",
            SlashCommand::ElevateSandbox => "设置增强型智能体沙箱",
            SlashCommand::Experimental => "启用或停用实验性功能",
            SlashCommand::AutoReview => "批准重试最近被自动审查拒绝的操作",
            SlashCommand::Memories => "配置记忆的使用和生成",
            SlashCommand::Mcp => "列出已配置的 MCP 工具；使用 /mcp verbose 查看详情",
            SlashCommand::Apps => "管理应用",
            SlashCommand::Plugins => "浏览插件",
            SlashCommand::Logout => "退出 Codex 登录",
            SlashCommand::Rollout => "输出 rollout 文件路径",
            SlashCommand::TestApproval => "测试审批请求",
        }
    }

    /// Command string without the leading '/'. Provided for compatibility with
    /// existing code that expects a method named `command()`.
    pub fn command(self) -> &'static str {
        self.into()
    }

    /// Whether this command supports inline args (for example `/review ...`).
    pub fn supports_inline_args(self) -> bool {
        matches!(
            self,
            SlashCommand::Review
                | SlashCommand::Rename
                | SlashCommand::New
                | SlashCommand::Clear
                | SlashCommand::Fork
                | SlashCommand::Plan
                | SlashCommand::Goal
                | SlashCommand::Voice
                | SlashCommand::Ide
                | SlashCommand::Keymap
                | SlashCommand::Mcp
                | SlashCommand::Export
                | SlashCommand::Raw
                | SlashCommand::Cd
                | SlashCommand::Pwd
                | SlashCommand::Usage
                | SlashCommand::Pets
                | SlashCommand::Side
                | SlashCommand::Btw
                | SlashCommand::Resume
        )
    }

    /// Whether this command remains available inside an active side conversation.
    pub fn available_in_side_conversation(self) -> bool {
        matches!(
            self,
            SlashCommand::Copy
                | SlashCommand::Agents
                | SlashCommand::Export
                | SlashCommand::Raw
                | SlashCommand::Diff
                | SlashCommand::Mention
                | SlashCommand::Status
                | SlashCommand::Daemon
                | SlashCommand::Warnings
                | SlashCommand::Pwd
                | SlashCommand::Usage
                | SlashCommand::Ide
        )
    }

    /// Whether dispatch needs thread state to validate this command before consuming its draft.
    /// The composer must defer busy-state rejection and draft clearing for these commands.
    pub(crate) fn requires_dispatch_validation(self) -> bool {
        matches!(self, SlashCommand::Review)
    }

    /// Commands that do not require a writable current thread. The server must still be connected.
    pub(crate) fn available_when_thread_unavailable(self) -> bool {
        matches!(
            self,
            SlashCommand::New
                | SlashCommand::Clear
                | SlashCommand::Resume
                | SlashCommand::Agents
                | SlashCommand::MultiAgents
                | SlashCommand::Quit
                | SlashCommand::Exit
                | SlashCommand::Status
                | SlashCommand::Warnings
                | SlashCommand::DebugConfig
                | SlashCommand::Pwd
                | SlashCommand::Rollout
                | SlashCommand::Copy
                | SlashCommand::Raw
        )
    }

    /// Whether this command can be run while a task is in progress.
    pub fn available_during_task(self) -> bool {
        match self {
            SlashCommand::New
            | SlashCommand::Archive
            | SlashCommand::Delete
            | SlashCommand::Fork
            | SlashCommand::Worktree
            | SlashCommand::Init
            | SlashCommand::Compact
            | SlashCommand::Recap
            | SlashCommand::Export
            | SlashCommand::Keymap
            | SlashCommand::Tui
            | SlashCommand::Vim
            | SlashCommand::ElevateSandbox
            | SlashCommand::Experimental
            | SlashCommand::Memories
            | SlashCommand::Import
            | SlashCommand::Review
            | SlashCommand::Plan
            | SlashCommand::Cd
            | SlashCommand::Clear
            | SlashCommand::Logout
            | SlashCommand::MemoryDrop
            | SlashCommand::MemoryUpdate => false,
            SlashCommand::Diff
            | SlashCommand::Resume
            | SlashCommand::Model
            | SlashCommand::Permissions
            | SlashCommand::Copy
            | SlashCommand::Raw
            | SlashCommand::Rename
            | SlashCommand::Mention
            | SlashCommand::Skills
            | SlashCommand::Hooks
            | SlashCommand::Status
            | SlashCommand::Daemon
            | SlashCommand::Warnings
            | SlashCommand::Pwd
            | SlashCommand::Usage
            | SlashCommand::DebugConfig
            | SlashCommand::Ps
            | SlashCommand::Stop
            | SlashCommand::App
            | SlashCommand::Goal
            | SlashCommand::Voice
            | SlashCommand::Mcp
            | SlashCommand::Apps
            | SlashCommand::Plugins
            | SlashCommand::Title
            | SlashCommand::Statusline
            | SlashCommand::AutoReview
            | SlashCommand::Feedback
            | SlashCommand::Ide
            | SlashCommand::Quit
            | SlashCommand::Exit
            | SlashCommand::Side
            | SlashCommand::Btw => true,
            SlashCommand::Rollout => true,
            SlashCommand::TestApproval => true,
            SlashCommand::Agents | SlashCommand::MultiAgents => true,
            SlashCommand::Theme | SlashCommand::Pets => false,
            // Switching providers rewrites `model`, which the running task depends on.
            SlashCommand::Provider => false,
        }
    }

    fn is_visible(self) -> bool {
        match self {
            SlashCommand::Copy => !cfg!(target_os = "android"),
            SlashCommand::App => cfg!(any(target_os = "macos", target_os = "windows")),
            SlashCommand::Voice => true,
            SlashCommand::Rollout | SlashCommand::TestApproval => cfg!(debug_assertions),
            _ => true,
        }
    }
}

/// Return all built-in commands in a Vec paired with their command string.
pub fn built_in_slash_commands() -> Vec<(&'static str, SlashCommand)> {
    SlashCommand::iter()
        .filter(|command| command.is_visible())
        .map(|c| (c.command(), c))
        .collect()
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use std::str::FromStr;

    use super::SlashCommand;

    #[test]
    fn stop_command_is_canonical_name() {
        assert_eq!(SlashCommand::Stop.command(), "stop");
    }

    #[test]
    fn clean_alias_parses_to_stop_command() {
        assert_eq!(SlashCommand::from_str("clean"), Ok(SlashCommand::Stop));
    }

    #[test]
    fn pet_alias_parses_to_pets_command() {
        assert_eq!(SlashCommand::Pets.command(), "pets");
        assert_eq!(SlashCommand::from_str("pet"), Ok(SlashCommand::Pets));
    }

    #[test]
    fn certain_commands_are_available_during_task() {
        assert!(SlashCommand::Goal.available_during_task());
        assert!(SlashCommand::Ide.available_during_task());
        assert!(SlashCommand::Title.available_during_task());
        assert!(SlashCommand::Statusline.available_during_task());
        assert!(SlashCommand::Raw.available_during_task());
        assert!(SlashCommand::Raw.available_in_side_conversation());
        assert!(SlashCommand::Raw.supports_inline_args());
        assert!(SlashCommand::App.available_during_task());
    }

    #[test]
    fn auto_review_command_is_approve() {
        assert_eq!(SlashCommand::AutoReview.command(), "approve");
        assert_eq!(
            SlashCommand::from_str("approve"),
            Ok(SlashCommand::AutoReview)
        );
    }
}

use serde::{Deserialize, Serialize};

/// メンションの種類
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MentionType {
    /// ファイルパス
    File,
    /// フォルダパス
    Folder,
    /// URL
    Url,
    /// ワークスペースの問題
    Problems,
    /// Git変更
    GitChanges,
    /// Gitコミット
    GitCommit,
}

/// メンションの内容
#[derive(Debug, Clone)]
pub struct MentionContent {
    /// メンションの種類
    pub mention_type: MentionType,
    /// メンションの値（パスやURL）
    pub value: String,
    /// 追加の説明（エラーメッセージなど）
    pub description: Option<String>,
}

/// メンション処理の結果
#[derive(Debug)]
pub struct ParsedMention {
    /// 元のテキスト
    pub original: String,
    /// 置換後のテキスト
    pub replacement: String,
    /// メンションの内容
    pub content: MentionContent,
}

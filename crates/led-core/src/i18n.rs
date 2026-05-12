use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;
use crate::config::Config;

#[derive(Clone)]
pub struct I18n {
    strings: HashMap<String, String>,
}

impl I18n {
    pub fn load(lang: &str) -> Self {
        let mut strings = if lang == "ja" {
            Self::get_ja_defaults()
        } else {
            Self::get_en_defaults()
        };
        
        // Always load English as absolute base
        if lang != "en" {
            let en = Self::get_en_defaults();
            for (k, v) in en {
                strings.entry(k).or_insert(v);
            }
        }

        if let Some(path) = Self::locale_file_path(lang) {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(path) {
                    if let Ok(custom) = Self::parse_locale_toml(&content) {
                        for (k, v) in custom {
                            strings.insert(k, v);
                        }
                    }
                }
            }
        }
        
        Self { strings }
    }

    fn get_en_defaults() -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("menu.file".to_string(), "File".to_string());
        m.insert("menu.file.new".to_string(), "New".to_string());
        m.insert("menu.file.open".to_string(), "Open…".to_string());
        m.insert("menu.file.save".to_string(), "Save".to_string());
        m.insert("menu.file.save_as".to_string(), "Save As…".to_string());
        m.insert("menu.file.close".to_string(), "Close".to_string());
        m.insert("menu.file.exit".to_string(), "Exit".to_string());

        m.insert("menu.edit".to_string(), "Edit".to_string());
        m.insert("menu.edit.undo".to_string(), "Undo".to_string());
        m.insert("menu.edit.redo".to_string(), "Redo".to_string());
        m.insert("menu.edit.cut".to_string(), "Cut".to_string());
        m.insert("menu.edit.copy".to_string(), "Copy".to_string());
        m.insert("menu.edit.paste".to_string(), "Paste".to_string());
        m.insert("menu.edit.find".to_string(), "Find…".to_string());
        m.insert("menu.edit.replace".to_string(), "Replace…".to_string());
        m.insert("menu.edit.select_all".to_string(), "Select All".to_string());

        m.insert("menu.view".to_string(), "View".to_string());
        m.insert("menu.view.go_to_line".to_string(), "Go to Line…".to_string());
        m.insert("menu.view.line_numbers".to_string(), "Line Numbers".to_string());
        m.insert("menu.view.word_wrap".to_string(), "Word Wrap".to_string());
        m.insert("menu.view.vi_mode".to_string(), "Vi Mode".to_string());
        m.insert("menu.view.encoding".to_string(), "Encoding".to_string());
        m.insert("menu.view.line_ending".to_string(), "Line Ending".to_string());
        m.insert("menu.view.theme".to_string(), "Theme".to_string());
        m.insert("menu.view.syntax".to_string(), "Syntax".to_string());

        m.insert("menu.help".to_string(), "Help".to_string());
        m.insert("menu.help.about".to_string(), "About".to_string());

        m.insert("panel.find".to_string(), "Find:".to_string());
        m.insert("panel.replace".to_string(), "Replace:".to_string());
        m.insert("panel.prev".to_string(), "< Prev".to_string());
        m.insert("panel.next".to_string(), "> Next".to_string());
        m.insert("panel.replace_one".to_string(), "Replace".to_string());
        m.insert("panel.replace_all".to_string(), "Replace All".to_string());
        m.insert("panel.close".to_string(), "Close".to_string());
        m.insert("panel.match_case".to_string(), "Match Case".to_string());
        m.insert("panel.whole_word".to_string(), "Whole Word".to_string());
        m.insert("panel.use_regex".to_string(), "Use Regex".to_string());

        m.insert("status.no_name".to_string(), "[No Name]".to_string());
        m.insert("status.no_matches".to_string(), "No matches".to_string());
        m.insert("status.search_wrapped_top".to_string(), "Search wrapped to top".to_string());
        m.insert("status.search_wrapped_bottom".to_string(), "Search wrapped to bottom".to_string());
        m.insert("status.matches".to_string(), "{current} of {total} matches".to_string());
        m.insert("status.replaced_count".to_string(), "{n} replacement(s) made".to_string());
        m.insert("status.terminal_too_small".to_string(), "Terminal too small ({cols}x{rows}). Please resize.".to_string());
        m.insert("status.cursor".to_string(), "Ln {line}, Col {col}".to_string());
        m.insert("status.selection".to_string(), "{n} chars".to_string());

        m.insert("error".to_string(), "Error".to_string());
        m.insert("error.cannot_open_dir".to_string(), "Cannot open directory: {path}".to_string());
        m.insert("error.failed_to_open".to_string(), "Failed to open {path}: {error}".to_string());

        m.insert("dialog.ok".to_string(), "OK".to_string());
        m.insert("dialog.cancel".to_string(), "Cancel".to_string());
        m.insert("dialog.yes".to_string(), "Yes".to_string());
        m.insert("dialog.no".to_string(), "No".to_string());
        m.insert("dialog.save".to_string(), "Save".to_string());
        m.insert("dialog.dont_save".to_string(), "Don't Save".to_string());
        m.insert("dialog.discard_reopen".to_string(), "Discard & Reopen".to_string());
        m.insert("dialog.discard_reopen_prompt".to_string(), "Discard unsaved changes and reopen?".to_string());
        m.insert("dialog.reopen_file".to_string(), "Reopen File".to_string());
        m.insert("dialog.open_file".to_string(), "Open File…".to_string());
        m.insert("dialog.save_as".to_string(), "Save As…".to_string());
        m.insert("dialog.go_to_line".to_string(), "Go to Line".to_string());
        m.insert("dialog.about".to_string(), "About".to_string());
        m.insert("dialog.show_hidden".to_string(), "Show Hidden".to_string());
        m.insert("dialog.detect_encoding".to_string(), "Detect Encoding".to_string());
        m.insert("dialog.overwrite_prompt".to_string(), "File already exists. Overwrite?".to_string());
        m.insert("dialog.unsaved_changes_title".to_string(), "Unsaved Changes".to_string());
        m.insert("dialog.unsaved_changes".to_string(), "Unsaved changes in \"{filename}\".".to_string());

        m.insert("dialog.file_browser.name".to_string(), "Name".to_string());
        m.insert("dialog.file_browser.size".to_string(), "Size".to_string());
        m.insert("dialog.file_browser.modified".to_string(), "Modified".to_string());
        m.insert("dialog.file_browser.filename".to_string(), "File name".to_string());

        m.insert("menu.view.syntax_plain".to_string(), "Plain Text".to_string());

        m.insert("about.version".to_string(), "Version".to_string());
        m.insert("about.license".to_string(), "License".to_string());
        m
    }

    fn get_ja_defaults() -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("menu.file".to_string(), "ファイル".to_string());
        m.insert("menu.file.new".to_string(), "新規作成".to_string());
        m.insert("menu.file.open".to_string(), "開く…".to_string());
        m.insert("menu.file.save".to_string(), "保存".to_string());
        m.insert("menu.file.save_as".to_string(), "名前を付けて保存…".to_string());
        m.insert("menu.file.close".to_string(), "閉じる".to_string());
        m.insert("menu.file.exit".to_string(), "終了".to_string());

        m.insert("menu.edit".to_string(), "編集".to_string());
        m.insert("menu.edit.undo".to_string(), "元に戻す".to_string());
        m.insert("menu.edit.redo".to_string(), "やり直し".to_string());
        m.insert("menu.edit.cut".to_string(), "切り取り".to_string());
        m.insert("menu.edit.copy".to_string(), "コピー".to_string());
        m.insert("menu.edit.paste".to_string(), "貼り付け".to_string());
        m.insert("menu.edit.find".to_string(), "検索…".to_string());
        m.insert("menu.edit.replace".to_string(), "置換…".to_string());
        m.insert("menu.edit.select_all".to_string(), "すべて選択".to_string());

        m.insert("menu.view".to_string(), "表示".to_string());
        m.insert("menu.view.go_to_line".to_string(), "行移動…".to_string());
        m.insert("menu.view.line_numbers".to_string(), "行番号".to_string());
        m.insert("menu.view.word_wrap".to_string(), "右端で折り返す".to_string());
        m.insert("menu.view.vi_mode".to_string(), "Viモード".to_string());
        m.insert("menu.view.encoding".to_string(), "エンコード".to_string());
        m.insert("menu.view.line_ending".to_string(), "改行コード".to_string());
        m.insert("menu.view.theme".to_string(), "テーマ".to_string());
        m.insert("menu.view.syntax".to_string(), "シンタックス".to_string());

        m.insert("menu.help".to_string(), "ヘルプ".to_string());
        m.insert("menu.help.about".to_string(), "このソフトについて".to_string());

        m.insert("panel.find".to_string(), "検索:".to_string());
        m.insert("panel.replace".to_string(), "置換:".to_string());
        m.insert("panel.prev".to_string(), "前を検索".to_string());
        m.insert("panel.next".to_string(), "次を検索".to_string());
        m.insert("panel.replace_one".to_string(), "置換".to_string());
        m.insert("panel.replace_all".to_string(), "すべて置換".to_string());
        m.insert("panel.close".to_string(), "閉じる".to_string());
        m.insert("panel.match_case".to_string(), "大文字小文字を区別".to_string());
        m.insert("panel.whole_word".to_string(), "単語単位".to_string());
        m.insert("panel.use_regex".to_string(), "正規表現".to_string());

        m.insert("status.no_name".to_string(), "[無題]".to_string());
        m.insert("status.no_matches".to_string(), "見つかりません".to_string());
        m.insert("status.search_wrapped_top".to_string(), "最後まで検索したので先頭に戻りました".to_string());
        m.insert("status.search_wrapped_bottom".to_string(), "先頭まで検索したので最後に戻りました".to_string());
        m.insert("status.matches".to_string(), "{total} 個中 {current} 番目の一致".to_string());
        m.insert("status.replaced_count".to_string(), "{n} 箇所を置換しました".to_string());
        m.insert("status.terminal_too_small".to_string(), "ターミナルが小さすぎます ({cols}x{rows})。サイズを大きくしてください。".to_string());
        m.insert("status.cursor".to_string(), "{line} 行, {col} 列".to_string());
        m.insert("status.selection".to_string(), "{n} 文字選択".to_string());

        m.insert("error".to_string(), "エラー".to_string());
        m.insert("error.cannot_open_dir".to_string(), "ディレクトリは開けません: {path}".to_string());
        m.insert("error.failed_to_open".to_string(), "ファイルを読み込めませんでした {path}: {error}".to_string());

        m.insert("dialog.ok".to_string(), "OK".to_string());
        m.insert("dialog.cancel".to_string(), "キャンセル".to_string());
        m.insert("dialog.yes".to_string(), "はい".to_string());
        m.insert("dialog.no".to_string(), "いいえ".to_string());
        m.insert("dialog.save".to_string(), "保存".to_string());
        m.insert("dialog.dont_save".to_string(), "保存しない".to_string());
        m.insert("dialog.discard_reopen".to_string(), "破棄して再読み込み".to_string());
        m.insert("dialog.discard_reopen_prompt".to_string(), "変更を破棄して再読み込みしますか？".to_string());
        m.insert("dialog.reopen_file".to_string(), "再読み込み".to_string());
        m.insert("dialog.open_file".to_string(), "ファイルを開く…".to_string());
        m.insert("dialog.save_as".to_string(), "名前を付けて保存…".to_string());
        m.insert("dialog.go_to_line".to_string(), "行移動".to_string());
        m.insert("dialog.about".to_string(), "バージョン情報".to_string());
        m.insert("dialog.show_hidden".to_string(), "隠しファイルを表示".to_string());
        m.insert("dialog.detect_encoding".to_string(), "エンコードを自動判別".to_string());
        m.insert("dialog.overwrite_prompt".to_string(), "ファイルが既に存在します。上書きしますか？".to_string());
        m.insert("dialog.unsaved_changes_title".to_string(), "保存されていない変更".to_string());
        m.insert("dialog.unsaved_changes".to_string(), "\"{filename}\" は変更されています。保存しますか？".to_string());

        m.insert("dialog.file_browser.name".to_string(), "名前".to_string());
        m.insert("dialog.file_browser.size".to_string(), "サイズ".to_string());
        m.insert("dialog.file_browser.modified".to_string(), "更新日時".to_string());
        m.insert("dialog.file_browser.filename".to_string(), "ファイル名".to_string());

        m.insert("menu.view.syntax_plain".to_string(), "標準テキスト".to_string());

        m.insert("about.version".to_string(), "バージョン".to_string());
        m.insert("about.license".to_string(), "ライセンス".to_string());
        m
    }

    pub fn get<'a>(&'a self, key: &'a str) -> &'a str {
        self.strings.get(key).map(|s| s.as_str()).unwrap_or(key)
    }

    fn locale_file_path(lang: &str) -> Option<PathBuf> {
        Config::config_dir().map(|dir| dir.join("locales").join(format!("{}.toml", lang)))
    }

    fn parse_locale_toml(content: &str) -> anyhow::Result<HashMap<String, String>> {
        use toml_span::parse;
        let value = parse(content).map_err(|e| anyhow::anyhow!("failed to parse locale TOML: {:?}", e))?;
        let mut map = HashMap::new();
        if let Some(table) = value.as_table() {
            for (k, v) in table {
                Self::flatten_toml_value(&k.to_string(), v, &mut map);
            }
        }
        Ok(map)
    }

    fn flatten_toml_value(prefix: &str, value: &toml_span::Value, map: &mut HashMap<String, String>) {
        if let Some(s) = value.as_str() {
            map.insert(prefix.to_string(), s.to_string());
        } else if let Some(table) = value.as_table() {
            for (k, v) in table {
                let new_prefix = format!("{}.{}", prefix, k);
                Self::flatten_toml_value(&new_prefix, v, map);
            }
        }
    }
}

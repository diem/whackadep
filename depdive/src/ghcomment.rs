//! This module abstracts github comment generation
//! by using markdown, html, and emojis

#[derive(PartialEq)]
#[allow(dead_code)]
pub enum TextStyle {
    Plain,
    Bold,
    Italic,
    Code,
}

#[non_exhaustive]
pub enum Emoji {
    WhiteCheckMark,
    RedCross,
    Warning,
}

pub struct GitHubCommentGenerator {
    comment: String,
}

/// This type offers both functionality
/// to get makrdown formatted text just as string
/// or build a string along with the type
impl GitHubCommentGenerator {
    pub fn new() -> Self {
        Self {
            comment: String::new(),
        }
    }

    pub fn get_comment(&mut self) -> String {
        self.comment.clone()
    }

    pub fn append_comment(&mut self, s: &str) {
        self.comment.push_str(s);
    }

    pub fn add_text(&mut self, s: &str, style: &TextStyle) {
        self.append_comment(&Self::get_text(s, style));
    }

    pub fn get_text(s: &str, style: &TextStyle) -> String {
        match style {
            TextStyle::Plain => s.to_string(),
            TextStyle::Bold => format!("**{}**", s),
            TextStyle::Italic => format!("*{}*", s),
            TextStyle::Code => format!("` {} `", s),
        }
    }

    pub fn add_newline(&mut self, count: u8) {
        for _i in 0..count {
            self.comment.push('\n');
        }
    }

    pub fn add_bulleted_list<T: AsRef<str>>(&mut self, items: &[T], text_style: &TextStyle) {
        self.append_comment(&Self::get_bulleted_list(items, text_style));
        self.add_newline(2);
    }

    pub fn get_bulleted_list<T: AsRef<str>>(items: &[T], text_style: &TextStyle) -> String {
        let mut s = String::new();

        for item in items {
            s.push_str(&format!(
                "\n   * {}",
                Self::get_text(item.as_ref(), text_style)
            ))
        }

        s
    }

    pub fn add_collapsible_section(&mut self, title: &str, body: &str) {
        self.append_comment(&Self::get_collapsible_section(title, body));
        self.add_newline(2);
    }

    pub fn get_collapsible_section(title: &str, body: &str) -> String {
        format!(
            "<details>\n\t<summary>{}</summary><br>\n{}\n</details>",
            title, body
        )
    }

    pub fn get_emoji(emoji: Emoji) -> &'static str {
        match emoji {
            Emoji::WhiteCheckMark => ":white_check_mark:",
            Emoji::RedCross => ":x:",
            Emoji::Warning => ":warning:",
        }
    }

    pub fn add_header(&mut self, s: &str, level: usize) {
        self.append_comment(&Self::get_header_text(s, level));
        self.add_newline(1);
    }

    pub fn get_header_text(s: &str, level: usize) -> String {
        let mut header = String::new();
        for _i in 0..level {
            header.push('#')
        }
        header.push_str(&format!(" {}", s));
        header
    }

    pub fn get_checkmark(flag: bool) -> &'static str {
        match flag {
            true => Self::get_emoji(Emoji::WhiteCheckMark),
            false => Self::get_emoji(Emoji::RedCross),
        }
    }

    pub fn add_html_table<T: AsRef<str>>(&mut self, table: &[Vec<T>]) {
        self.add_newline(1);
        self.append_comment(&Self::get_html_table(table));
        self.add_newline(2);
    }

    pub fn get_html_table<T: AsRef<str>>(table: &[Vec<T>]) -> String {
        let mut s = String::new();
        s.push_str("<table>");
        for row in table {
            s.push_str("<tr>");
            for col in row {
                s.push_str("<td>");
                s.push_str(col.as_ref());
                s.push_str("</td>");
            }
            s.push_str("</tr>");
        }
        s.push_str("</table>");
        s
    }

    pub fn get_hyperlink(body: &str, url: &str) -> String {
        format!("[{}]({})", body, url)
    }
}

impl Default for GitHubCommentGenerator {
    fn default() -> Self {
        GitHubCommentGenerator::new()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_ghcomment_mutliline() {
        let test_str = "rust";
        let mut gh = GitHubCommentGenerator::new();
        gh.add_text(test_str, &TextStyle::Plain);
        gh.add_newline(1);
        gh.add_text(test_str, &TextStyle::Bold);
        gh.add_newline(1);
        gh.add_text(test_str, &TextStyle::Code);
        println!("{}", gh.get_comment());
        assert_eq!(gh.get_comment(), format!("{0}\n**{0}**\n` {0} `", test_str));
    }

    #[test]
    fn test_ghcomment_plain() {
        let test_str = "rust";
        let mut gh = GitHubCommentGenerator::new();
        gh.add_text(test_str, &TextStyle::Plain);
        assert_eq!(gh.get_comment(), test_str);
    }

    #[test]
    fn test_ghcomment_bold() {
        let test_str = "rust";
        let mut gh = GitHubCommentGenerator::new();
        gh.add_text(test_str, &TextStyle::Bold);
        assert_eq!(gh.get_comment(), format!("**{}**", test_str));
    }

    #[test]
    fn test_ghcomment_italic() {
        let test_str = "rust";
        let mut gh = GitHubCommentGenerator::new();
        gh.add_text(test_str, &TextStyle::Italic);
        assert_eq!(gh.get_comment(), format!("*{}*", test_str));
    }

    #[test]
    fn test_ghcomment_code() {
        let test_str = "rust";
        let mut gh = GitHubCommentGenerator::new();
        gh.add_text(test_str, &TextStyle::Code);
        assert_eq!(gh.get_comment(), format!("` {} `", test_str));
    }

    #[test]
    fn test_ghcomment_bulleted_list() {
        let mut gh = GitHubCommentGenerator::new();
        let v = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let s = "\n   * ` a `\n   * ` b `\n   * ` c `\n\n";
        gh.add_bulleted_list(&v, &TextStyle::Code);
        assert_eq!(gh.get_comment(), s);
    }

    #[test]
    fn test_ghcomment_collapsible_section() {
        let mut gh = GitHubCommentGenerator::new();
        let v = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let body = GitHubCommentGenerator::get_bulleted_list(&v, &TextStyle::Code);
        let title = "Click to expand!";
        gh.add_collapsible_section(title, &body);
        let s = "<details>\n\t<summary>Click to expand!</summary><br>\n\n   * ` a `\n   * ` b `\n   * ` c `\n</details>\n\n";
        assert_eq!(gh.get_comment(), s);
    }

    #[test]
    fn test_ghcomment_emoji() {
        let mut gh = GitHubCommentGenerator::new();
        let s = format!(
            "testing emoji {} {} {}",
            GitHubCommentGenerator::get_emoji(Emoji::WhiteCheckMark),
            GitHubCommentGenerator::get_emoji(Emoji::RedCross),
            GitHubCommentGenerator::get_emoji(Emoji::Warning)
        );
        gh.add_text(&s, &TextStyle::Plain);
        assert_eq!(
            gh.get_comment(),
            "testing emoji :white_check_mark: :x: :warning:"
        );
    }

    #[test]
    fn test_ghcomment_header() {
        let mut gh = GitHubCommentGenerator::new();
        let test_str = "rust";
        gh.add_header(test_str, 2);
        assert_eq!(gh.get_comment(), format!("## {}\n", test_str));
    }

    #[test]
    fn test_ghcomment_html_table() {
        let mut gh = GitHubCommentGenerator::new();
        gh.add_html_table(&[vec!["first", ":tada:"], vec!["first", ":tada:"]]);
        assert_eq!(gh.get_comment(),
        "\n<table><tr><td>first</td><td>:tada:</td></tr><tr><td>first</td><td>:tada:</td></tr></table>\n\n");
    }

    #[test]
    fn test_ghcomment_hyperlink() {
        assert_eq!(
            GitHubCommentGenerator::get_hyperlink("google", "www.google.com"),
            "[google](www.google.com)"
        );
    }
}

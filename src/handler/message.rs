use crate::{solved, util};
use num_format::{Locale, ToFormattedString};
use once_cell::sync::Lazy;
use regex::Regex;
use solved::ClassDecoration;
use telegram_bot::{Api, InputFileRef, Message, ParseMode, SendMessage, SendPhoto};

pub async fn answer_plain_message(bot: &Api, message: &Message, text: &str) -> anyhow::Result<()> {
    static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"#(\d{4,})|(\d{4,})번"#).unwrap());
    let mut result = String::new();
    for capture in REGEX.captures_iter(text) {
        if let Some(number) = capture.get(1).or_else(|| capture.get(2)) {
            let search = solved::search(number.as_str()).await?;
            if let Some(problem) = search.problems.first() {
                result.push_str(&format!(
                    "[{} \\| \\#{} \\- {}]({})\n",
                    problem.level,
                    problem.id,
                    util::escape_markdown(&problem.caption),
                    problem.href
                ));
            }
        }
    }
    result.pop();
    if !result.is_empty() {
        let mut reply = SendMessage::new(message.chat.id(), result);
        reply
            .reply_to(message)
            .disable_preview()
            .parse_mode(ParseMode::MarkdownV2);
        bot.send(reply).await?;
    }
    Ok(())
}

enum CommandType {
    Problem,
    User,
}

pub async fn answer_command<'a>(
    bot: &Api,
    message: &'a Message,
    command: Command<'a>,
) -> anyhow::Result<()> {
    use radix_trie::Trie;
    static COMMANDS: Lazy<Trie<&str, CommandType>> = Lazy::new(|| {
        let mut trie = Trie::new();
        trie.insert("problem", CommandType::Problem);
        trie.insert("user", CommandType::User);
        trie
    });
    if let Some(op) = COMMANDS.get_ancestor_value(command.label) {
        match op {
            CommandType::Problem => {
                let query = command.rest();
                let search = solved::search(query).await?;
                let mut result = String::new();
                for problem in search.problems {
                    result.push_str(&format!(
                        "[{} \\| \\#{} \\- {}]({})\n",
                        problem.level,
                        problem.id,
                        util::escape_markdown(&problem.caption),
                        problem.href
                    ));
                }
                result.pop();
                if result.is_empty() {
                    result.push_str("검색 결과가 없습니다\\.");
                }
                let mut reply = SendMessage::new(message.chat.id(), result);
                reply
                    .reply_to(message)
                    .disable_preview()
                    .parse_mode(ParseMode::MarkdownV2);
                bot.send(reply).await?;
            }
            CommandType::User => {
                let query = command.rest();
                let search = solved::search(query).await?;
                let mut result = String::new();
                let mut image = None;
                if let Some(user) = search.users.first() {
                    let class_decoration = match user.class_decoration {
                        ClassDecoration::Normal => "",
                        ClassDecoration::Silver => "\\+",
                        ClassDecoration::Gold => "\\+\\+",
                    };
                    result = format!(
                        "*{id} \\({}위\\)*\n\
                        __{}, Class {}{}__\n\
                        {}\n\
                        *{}문제* 해결 \\| *경험치* {}\n\
                        ▸ [solved\\.ac](https://solved.ac/profile/{id}) ▸ [acmicpc\\.net](https://acmicpc.net/user/{id})",
                        user.rank,
                        user.level,
                        user.class,
                        class_decoration,
                        util::escape_markdown(&user.bio),
                        user.solved.to_formatted_string(&Locale::en),
                        user.exp.to_formatted_string(&Locale::en),
                        id = util::escape_markdown(&user.user_id),
                    );
                    image = user.profile_image_url.clone();
                }
                if result.is_empty() {
                    result.push_str("검색 결과가 없습니다\\.");
                    let mut reply = SendMessage::new(message.chat.id(), result);
                    reply
                        .reply_to(message)
                        .disable_preview()
                        .parse_mode(ParseMode::MarkdownV2);
                    bot.send(reply).await?;
                } else {
                    let mut reply = SendPhoto::new(
                        message.chat.id(),
                        InputFileRef::new(image.as_deref().unwrap_or(
                            "https://static.solved.ac/misc/360x360/default_profile.png",
                        )),
                    );
                    reply
                        .caption(result)
                        .reply_to(message)
                        .parse_mode(ParseMode::MarkdownV2);
                    bot.send(reply).await?;
                }
            }
        }
    }
    Ok(())
}

pub struct Command<'a> {
    label: &'a str,
    args_left: &'a str,
}

impl<'a> Command<'a> {
    pub fn new(raw: &'a str) -> anyhow::Result<Self> {
        if !raw.starts_with('/') {
            return Err(anyhow::anyhow!(
                "Command must start with a slash, found: {}",
                raw
            ));
        }
        let delim = raw.find(char::is_whitespace);
        let new = if let Some(delim) = delim {
            Self {
                label: &raw[1..delim],
                args_left: raw[delim + 1..].trim_start(),
            }
        } else {
            Self {
                label: raw,
                args_left: "",
            }
        };
        Ok(new)
    }
    pub fn rest(&self) -> &str {
        self.args_left
    }
}

impl<'a> Iterator for Command<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(delim) = self.args_left.find(char::is_whitespace) {
            let arg = &self.args_left[..delim];
            self.args_left = self.args_left[delim + 1..].trim_start();
            Some(arg)
        } else if self.args_left.is_empty() {
            None
        } else {
            let arg = self.args_left;
            self.args_left = "";
            Some(arg)
        }
    }
}

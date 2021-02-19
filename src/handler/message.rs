use crate::solved;
use num_format::{Locale, ToFormattedString};
use once_cell::sync::Lazy;
use regex::Regex;
use solved::ClassDecoration;
use tgbot::types::{InputFile, Message, ParseMode};
use tgbot::{
    methods::{SendMessage, SendPhoto},
    Api,
};

pub async fn answer_plain_message(bot: &Api, message: &Message, text: &str) -> anyhow::Result<()> {
    static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"#(\d{4,})|(\d{4,})번"#).unwrap());
    let mut result = String::new();
    let matches: Vec<_> = REGEX
        .captures_iter(text)
        .filter_map(|c| c.get(1).or_else(|| c.get(2)))
        .collect();
    for number in matches {
        let number = number.as_str();
        let number_int = number.parse().unwrap_or(0);
        let search = solved::search(number).await?;
        if let Some(problem) = search
            .problems
            .iter()
            .find(|problem| problem.id == number_int)
        {
            result.push_str(&format!(
                "[{} \\| \\#{} \\- {}]({})\n",
                problem.level,
                problem.id,
                ParseMode::MarkdownV2.escape(&problem.caption),
                problem.href
            ));
        }
    }
    result.pop();
    if !result.is_empty() {
        let reply = SendMessage::new(message.get_chat_id(), result)
            .reply_to_message_id(message.id)
            .disable_web_page_preview(true)
            .parse_mode(ParseMode::MarkdownV2);
        bot.execute(reply).await?;
    }
    Ok(())
}

mod command_type {
    pub const PROBLEM: u64 = 1;
    pub const USER: u64 = 2;
}

pub async fn answer_command<'a>(
    bot: &Api,
    message: &'a Message,
    command: Command<'a>,
) -> anyhow::Result<()> {
    use fst::automaton::Subsequence;
    use fst::{IntoStreamer, Map, MapBuilder, Streamer};
    static COMMANDS: Lazy<Map<Vec<u8>>> = Lazy::new(|| {
        let mut map = MapBuilder::memory();
        map.insert("problem", command_type::PROBLEM).unwrap();
        map.insert("user", command_type::USER).unwrap();
        map.into_map()
    });
    let matcher = Subsequence::new(command.label);
    if let Some((_key, value)) = COMMANDS.search(matcher).into_stream().next() {
        match value {
            command_type::PROBLEM => {
                let query = command.rest();
                let search = solved::search(query).await?;
                let mut result = String::new();
                for problem in search.problems {
                    result.push_str(&format!(
                        "[{} \\| \\#{} \\- {}]({})\n",
                        problem.level,
                        problem.id,
                        ParseMode::MarkdownV2.escape(&problem.caption),
                        problem.href
                    ));
                }
                result.pop();
                if result.is_empty() {
                    result.push_str("검색 결과가 없습니다\\.");
                }
                let reply = SendMessage::new(message.get_chat_id(), result)
                    .reply_to_message_id(message.id)
                    .disable_web_page_preview(true)
                    .parse_mode(ParseMode::MarkdownV2);
                bot.execute(reply).await?;
            }
            command_type::USER => {
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
                        ParseMode::MarkdownV2.escape(&user.bio),
                        user.solved.to_formatted_string(&Locale::en),
                        user.exp.to_formatted_string(&Locale::en),
                        id = ParseMode::MarkdownV2.escape(&user.user_id),
                    );
                    image = user.profile_image_url.clone();
                }
                if result.is_empty() {
                    result.push_str("검색 결과가 없습니다\\.");
                    let reply = SendMessage::new(message.get_chat_id(), result)
                        .reply_to_message_id(message.id)
                        .disable_web_page_preview(true)
                        .parse_mode(ParseMode::MarkdownV2);
                    bot.execute(reply).await?;
                } else {
                    let reply = SendPhoto::new(
                        message.get_chat_id(),
                        InputFile::url(image.as_deref().unwrap_or(
                            "https://static.solved.ac/misc/360x360/default_profile.png",
                        )),
                    )
                    .caption(result)
                    .reply_to_message_id(message.id)
                    .parse_mode(ParseMode::MarkdownV2);
                    bot.execute(reply).await?;
                }
            }
            _ => {}
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

use crate::{solved, util};
use anyhow::Context;
use telegram_bot::{
    AnswerInlineQuery, Api, InlineQuery, InlineQueryResult, InlineQueryResultArticle,
    InputTextMessageContent, ParseMode,
};

pub async fn answer_inline_query(bot: &Api, query: InlineQuery) -> anyhow::Result<()> {
    let search_result = solved::search(&query.query)
        .await
        .with_context(|| format!("Query: {}", &query.query))?;
    let inline_results = search_result
        .problems
        .iter()
        .map(problem_to_query_result)
        .chain(search_result.users.iter().map(user_to_query_result))
        .collect();
    let task = AnswerInlineQuery::new(query.id, inline_results);
    bot.send(task).await?;
    Ok(())
}

fn problem_to_query_result(problem: &solved::Problem) -> InlineQueryResult {
    let id = problem.id.to_string();
    let title = format!("{}번 - {}", &id, problem.caption);
    let text = format!(
        "[{} \\| \\#{} \\- {}\n{}명 해결]({})",
        problem.level,
        &id,
        util::escape_markdown(&problem.caption),
        problem.solved,
        problem.href
    );
    let content = InputTextMessageContent {
        message_text: text,
        parse_mode: Some(ParseMode::MarkdownV2),
        disable_web_page_preview: true,
    };
    InlineQueryResultArticle::new(id, title, content).into()
}

use num_format::{Locale, ToFormattedString};
fn user_to_query_result(user: &solved::User) -> InlineQueryResult {
    use solved::ClassDecoration;
    let class_decoration = match user.class_decoration {
        ClassDecoration::Normal => "",
        ClassDecoration::Silver => "\\+",
        ClassDecoration::Gold => "\\+\\+",
    };
    let text = format!(
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
        id = user.user_id,
    );
    InlineQueryResultArticle::new(
        &user.user_id,
        &user.user_id,
        InputTextMessageContent {
            message_text: text,
            parse_mode: Some(ParseMode::MarkdownV2),
            disable_web_page_preview: true,
        },
    )
    .into()
}

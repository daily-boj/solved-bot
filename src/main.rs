use actix_web::{middleware, web, App, HttpResponse, HttpServer};
use log::error;
use telegram_bot::{Api, MessageKind, Update};
mod handler;
mod solved;
mod util;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();
    HttpServer::new(move || {
        let token = std::env::var("BOT_TOKEN").expect("Could not find BOT_TOKEN");
        let api = telegram_bot::Api::new(&token);
        App::new()
            .data(api)
            .wrap(middleware::Logger::default())
            .data(web::JsonConfig::default().limit(8192))
            .default_service(web::post().to(webhook))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

async fn webhook(bot: web::Data<Api>, msg: web::Json<Update>) -> HttpResponse {
    if let Err(e) = handle_update(&bot, msg.0).await {
        error!("An error occurred processing webhook: {:?}", e);
    }
    HttpResponse::Ok().finish()
}

async fn handle_update(bot: &Api, update: Update) -> anyhow::Result<()> {
    use telegram_bot::UpdateKind;
    match update.kind {
        UpdateKind::Message(ref message) => {
            if message.forward.is_some() {
                return Ok(());
            }
            match message.kind {
                MessageKind::Text { ref data, .. } => {
                    if data.starts_with('/') {
                        handler::answer_command(bot, message, handler::Command::new(data)?).await?;
                    } else {
                        handler::answer_plain_message(bot, message, data).await?;
                    }
                }
                _ => {}
            }
        }
        UpdateKind::InlineQuery(query) => handler::answer_inline_query(bot, query).await?,
        _ => {}
    }
    Ok(())
}

use actix_web::{middleware, web, App, HttpResponse, HttpServer};
use log::error;
use telegram_bot::{Api, Update};
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
            .service(web::resource(&token).route(web::post().to(webhook)))
            .default_service(web::to(not_found))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

fn not_found() -> HttpResponse {
    HttpResponse::NotFound().finish()
}

async fn webhook(bot: web::Data<Api>, msg: web::Json<Update>) -> HttpResponse {
    match handle_update(&bot, msg.0).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            error!("An error occurred processing webhook: {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

async fn handle_update(bot: &Api, update: Update) -> anyhow::Result<()> {
    use telegram_bot::UpdateKind;
    if let UpdateKind::InlineQuery(query) = update.kind {
        handler::answer_inline_query(bot, query).await?;
    }
    Ok(())
}

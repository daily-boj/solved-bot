use anyhow::Context;
use log::{error, info};
use tgbot::{async_trait, types::Update, webhook, Api, Config, UpdateHandler};
mod handler;
mod solved;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();
    let token = std::env::var("BOT_TOKEN").expect("Could not find BOT_TOKEN");
    let api = Api::new(Config::new(token)).expect("Could not create API");
    let bind_address = std::env::var("BOT_BIND_ADDRESS").expect("Could not find BOT_BIND_ADDRESS");
    let socket_addr = bind_address.parse::<std::net::SocketAddr>().unwrap();
    info!("webhook running on {}", bind_address);
    webhook::run_server(socket_addr, "/", Handler { api })
        .await
        .unwrap();
}

struct Handler {
    api: Api,
}

#[async_trait]
impl UpdateHandler for Handler {
    async fn handle(&mut self, update: Update) {
        info!("Incoming update {:?}", &update);
        if let Err(e) = handle_update(&self.api, update).await {
            error!("An error occurred processing webhook: {:?}", e);
        }
    }
}

async fn handle_update(bot: &Api, update: Update) -> anyhow::Result<()> {
    use tgbot::types::{MessageData, UpdateKind};
    match update.kind {
        UpdateKind::Message(ref message) => {
            if message.via_bot.is_some() {
                return Ok(());
            }
            match message.data {
                MessageData::Text(ref text) => {
                    if text.data.starts_with('/') {
                        handler::answer_command(bot, message, handler::Command::new(&text.data)?)
                            .await
                            .with_context(|| text.data.clone())?;
                    } else {
                        handler::answer_plain_message(bot, message, &text.data)
                            .await
                            .with_context(|| text.data.clone())?;
                    }
                }
                _ => {}
            }
        }
        UpdateKind::InlineQuery(query) => handler::answer_inline_query(bot, &query)
            .await
            .with_context(|| query.query.clone())?,
        _ => {}
    }
    Ok(())
}

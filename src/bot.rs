use std::{env, thread, sync::Arc};

use async_runtime::rt;
use serenity::{prelude::{GatewayIntents, EventHandler, Context}, Client, async_trait, model::prelude::{Message, Ready, Activity}, utils::MessageBuilder, futures::lock::Mutex};

use crate::api;

const PREFIX: &str = "!";
const MAIN_GUILD_ID: u64 = 1034388873465299014;

const EXECUTIVE_ADMIN_ID: u64 = 1034391740481814589;

#[derive(Clone)]
pub struct Bot {
    pub context: Arc<Context>
}

impl Bot {
    fn new(context: Context) -> Self {
        Self {
            context: Arc::new(context)
        }
    }

    pub fn set_offline(&mut self, offline: bool) {
        println!("Setting status to {:?}", if offline { "offline" } else { "online" });

        let channel = Channel::new(self.context.clone(), 1034413806392197181);

        if offline {
            channel.send_message("Server offline".to_string());
            let ctx = self.context.clone();
            thread::spawn(move || {
                ctx.shard.set_activity(Some(Activity::playing("Server offline")));
            });
        } else {
            channel.send_message("Server online".to_string());
        }
    }

    pub fn update_player(&mut self, player_count: isize) {
        let ctx = self.context.clone();
        thread::spawn(move || {
            ctx.shard.set_activity(Some(Activity::playing(format!("Server online {}/48", player_count))));
        });
    }
}

struct Channel {
    context: Arc<Context>,
    channel_id: Arc<u64>,
}

impl Channel {
    fn new(context: Arc<Context>, channel_id: u64) -> Self {
        Self {
            context,
            channel_id: Arc::new(channel_id),
        }
    }

    fn send_message(&self, message: String) {
        let ctx = self.context.clone();
        let channel_id = self.channel_id.clone();
        rt::spawn(async move {
            let message = MessageBuilder::new()
                .push(message)
                .build();

            ctx.http.get_channel(*channel_id).await.unwrap().id().say(&ctx.http, message).await.unwrap();
        }).unwrap();
    }
}

struct MessageCommand {
    context: Arc<Context>,
}

impl MessageCommand {
    fn handle(context: Context, message: Message) {
        let own = Arc::new(Mutex::new(Self { context: Arc::new(context) }));

        {
            let own = own.clone();
            rt::spawn(async move {
                let mut own = own.lock().await;

                let is_admin = message.author.has_role(&own.context.http, MAIN_GUILD_ID, EXECUTIVE_ADMIN_ID).await.unwrap();

                if is_admin {
                    own.admin(message);
                }
            }).unwrap();
        }
    }

    fn admin(&mut self, message: Message) {
        let msg = message.clone();
        let content = message.content;
        let prefix = content.get(0..1).unwrap().to_owned();

        if prefix == PREFIX {
            let content = content.clone();
            let content = content.get(1..content.len()).unwrap();
            let content: Vec<&str> = content.split(" ").collect();
            let content: Vec<String> = content.iter().copied().map(String::from).collect();
            
            let command = content.get(0).unwrap().to_owned();
            let arguments = content.get(1..).unwrap().to_owned();

            println!("Command : {:?}, Arguments: {:?}", command, arguments);

            let context = self.context.clone();
            let message = Arc::new(msg);
            match command.as_str() {
                "echo" => {
                    if arguments.len() <= 0 { return; }
                    let channel = Channel::new(context, message.channel_id.0);

                    channel.send_message(arguments.join(" "));
                },
                "test" => {
                    rt::spawn(async move {
                        message.reply(&context.http, arguments.join(" ")).await.unwrap();
                    }).unwrap();
                },
                _ => {}
            }
        }
    }
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, _ctx: Context, _msg: Message) {
        MessageCommand::handle(_ctx, _msg);
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        api::launch(Bot::new(ctx));

        println!("{} is connected!", ready.user.name);
    }
}

pub fn launch() {
    tokio::spawn(async {
        let token = env::var("TOKEN").expect("Expected a token in the Environment");
        let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;
        let mut client = Client::builder(&token, intents).event_handler(Handler).await.expect("Err creating client");

        if let Err(why) = client.start().await {
            eprintln!("Client error: {:?}", why);
        }
    });
}
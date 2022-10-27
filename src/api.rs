use std::{convert::Infallible, net::{SocketAddr}, time::Duration, sync::{Arc, Mutex, atomic::{AtomicBool, AtomicI8}, mpsc::{channel, Sender}}, thread, ops::AddAssign, collections::HashMap};

use chrono::{DateTime, Utc, Timelike};
use hyper::{Request, Body, Response, service::{service_fn, make_service_fn}, Server, server::conn::AddrStream};

use crate::bot::{Bot};

pub fn launch(bot: Bot) {
    let api = Arc::new(API::new(bot));

    tokio::spawn(async move {
        let address = SocketAddr::from(([127, 0, 0, 1], 30180));

        let make_svc = make_service_fn(move |_conn: &AddrStream| {
            let api = api.clone();
            async move {
                Ok::<_, Infallible>(service_fn(move |req| {
                    let api = api.clone();
                    async move {
                        api.init(req).await
                    }
                }))
            }
        });

        let server = Server::bind(&address).serve(make_svc);
        
        println!("API server started!");
        if let Err(e) = server.await {
            eprintln!("{:?}", e);
        }
    });
}

struct API {
    last_heartbeat: Arc<Mutex<DateTime<Utc>>>,
    bot: Arc<Mutex<Bot>>,
    offline: Arc<Mutex<bool>>,
    data: Arc<Mutex<HashMap<String, String>>>,
    update: Arc<Mutex<bool>>,
}

impl API {
    async fn init(&self, req: Request<Body>) -> Result<Response<Body>, Infallible> {
        let mut path: Vec<&str> = req.uri().path().split('/').collect();
        path.remove(0);

        if path.get(0).unwrap().to_owned() == "heartbeat" {
            self.heartbeat()
        } else if path.get(0).unwrap().to_owned() == "player" {
            self.update_player_count(path.get(1).unwrap().to_string())
        } else {
            Ok(Response::new("404 Not found".into()))
        }
    }

    fn new(bot: Bot) -> Self {
        let last_heartbeat = Arc::new(Mutex::new(Utc::now()));
        let offline = Arc::new(Mutex::new(true));
        let update = Arc::new(Mutex::new(true));
        let bot = Arc::new(Mutex::new(bot));
        let data: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));

        {
            let last_heartbeat = Arc::clone(&last_heartbeat);
            let update = Arc::clone(&update);
            let offline = Arc::clone(&offline);
            thread::spawn(move || {
                loop {
                    let last_heartbeat = last_heartbeat.lock().unwrap();
                    let mut off = offline.lock().unwrap();

                    if Utc::now().timestamp() >= (last_heartbeat.timestamp() + 10) {
                        println!("Heartbeat timeout");

                        let mut update = update.lock().unwrap();
                        *update = true;
                        *off = true;

                        drop(update);
                        drop(off);
                        drop(last_heartbeat);

                        loop {
                            let offline = offline.lock().unwrap();
                            if !*offline {
                                break;
                            }
                        }
                    } else {
                        *off = false;
                    }
                }
            });
        }

        {
            let last_heartbeat = Arc::clone(&last_heartbeat);
            let update = Arc::clone(&update);
            let offline = Arc::clone(&offline);
            let bot = Arc::clone(&bot);
            let data = Arc::clone(&data);
            thread::spawn(move || {
                let times = AtomicI8::new(0);
                loop {
                    let last_heartbeat = last_heartbeat.lock().unwrap();
                    let mut update = update.lock().unwrap();
                    let mut offline = offline.lock().unwrap();
                    let bot = bot.lock().unwrap();
                    let data = data.lock().unwrap();

                    if *update {
                        times.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        let mut bot = bot.clone();
                        let data = data.clone();
                        if *offline {
                            if Utc::now().timestamp() >= (last_heartbeat.timestamp() + 10) {
                                bot.set_offline(true);
                            } else {
                                bot.set_offline(false);
                                *offline = false;
                            }
                        } else {
                            bot.update_player(data.get("player").unwrap_or(&"0".to_string()).parse().unwrap());
                        }
                        *update = false;
                    }
                }
            });
        }

        API {
            last_heartbeat,
            bot,
            offline,
            data,
            update,
        }
    }

    fn heartbeat(&self) -> Result<Response<Body>, Infallible> {
        println!("Receiving heartbeat from server");

        let mut last_heartbeat = self.last_heartbeat.lock().unwrap();
        let mut update = self.update.lock().unwrap();
        *last_heartbeat = Utc::now();
        *update = true;
        drop(update);
        drop(last_heartbeat);

        Ok(Response::new("heartbeat ok".into()))
    }

    fn update_player_count(&self, count: String) -> Result<Response<Body>, Infallible> {
        let count: usize = count.parse().expect("Count is not a number");

        println!("Updating player count to {:?}", count);

        drop(self.bot.lock().unwrap().update_player(count as isize));
        let mut data = self.data.lock().unwrap();
        data.insert("player".to_string(), count.to_string());
        drop(data);

        Ok(Response::new("update player ok".into()))
    }
}
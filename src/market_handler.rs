use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex
};

use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{broadcast, mpsc};
use tokio::time::{sleep, Duration};

use crate::utils;
use crate::errors;
use crate::manifold_types;

#[derive(Serialize, Deserialize, Debug, Clone)]
enum Method {
    GET,
    POST,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PostyPacket {
    bot_id: String,
    method: Method,
    endpoint: String,
    query_params: Vec<(String, String)>,
    data: Option<Value>,
    response: String,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct MarketHandler {
    api_read_limit_per_s: u32,
    api_write_limit_per_min: u32,
    halt_flag: Arc<AtomicBool>,

    bots_to_mh_tx: mpsc::Sender<PostyPacket>,
    // mh_to_bots_rx: mpsc::Receiver<PostyPacket>,

    bot_out_channel: Arc<Mutex<HashMap<String, broadcast::Sender<PostyPacket>>>>,

    bet_channels: HashMap<String, broadcast::Sender<manifold_types::Bet>>,
}

#[allow(dead_code)]
impl MarketHandler {
    pub fn new() -> Self {
        let halt_flag = Arc::new(AtomicBool::new(false));

        let (bots_to_mh_tx, mut bots_to_mh_rx) = mpsc::channel::<PostyPacket>(256);
        // let bot_out_channel: HashMap<String, broadcast::Sender<PostyPacket>> = HashMap::new();
        let bot_out_channel: Arc<Mutex<HashMap<String, broadcast::Sender<PostyPacket>>>> = Arc::new(Mutex::new(HashMap::new()));

        let halt_flag_clone = halt_flag.clone();
        let bot_out_channel_clone = bot_out_channel.clone();

        tokio::spawn(async move {
            while !halt_flag_clone.load(Ordering::SeqCst) {

                // why a Option instead of a Result here?
                let posty_packet = match bots_to_mh_rx.recv().await {
                    Some(packet) => packet,
                    None => {
                        warn!("posty packet rx is none");
                        continue;
                    }
                };

                let maybe_res = match posty_packet.method {
                    Method::GET => get_endpoint(posty_packet.endpoint.clone(), &posty_packet.query_params).await,
                    Method::POST => post_endpoint(posty_packet.endpoint.clone(), &posty_packet.query_params, posty_packet.data).await
                };

                let res = match maybe_res {
                    Ok(res) => res,
                    Err(e) => {
                        error!("api error {e}");
                        continue;
                    }
                }.text().await.unwrap();

                let bot_id = posty_packet.bot_id;
                bot_out_channel_clone.lock().unwrap().get(&bot_id).unwrap().send(PostyPacket {
                    bot_id,
                    method: posty_packet.method,
                    endpoint: posty_packet.endpoint,
                    query_params: posty_packet.query_params,
                    data: None,
                    response: res,
                }).expect("couldn't send posty packet");
            }
        });

        Self {
            api_read_limit_per_s: 100,
            api_write_limit_per_min: 10,
            halt_flag,
            bots_to_mh_tx,
            // mh_to_bots_rx,
            bot_out_channel,
            bet_channels: HashMap::new(),
        }
    }

    pub fn halt(&self) {
        self.halt_flag.store(true, Ordering::SeqCst);
    }

    pub async fn check_alive(&self) -> bool {
        let resp = get_endpoint("me".to_string(), &[]).await.unwrap();

        resp.json::<manifold_types::User>().await.is_ok()
    }

    pub async fn whoami(&self) -> manifold_types::User {
        let resp = get_endpoint("me".to_string(), &[]).await.unwrap();

        resp.json::<manifold_types::User>().await.unwrap()
    }

    pub async fn market_search(
        &self,
        term: String,
    ) -> Result<manifold_types::FullMarket, errors::ReqwestResponseParsing> {
        let resp = get_endpoint(
            "search-markets".to_string(),
            &[
                ("term".to_string(), term.clone()),
                ("limit".to_string(), "1".to_string()),
            ],
        )
        .await
        .unwrap();

        let lite_market_req = response_into::<Vec<manifold_types::LiteMarket>>(resp).await;
        let lite_market = match lite_market_req {
            Ok(mut markets) => {
                if markets.len() == 1 {
                    Ok(markets.pop())
                } else {
                    error!("no markets found for term {}", &term);
                    Err(errors::ReqwestResponseParsing::APIGeneric(format!(
                        "no markets found for term {}",
                        &term
                    )))
                }
            }
            Err(e) => Err(e),
        }?;

        let full_market =
            get_endpoint(format!("market/{}", lite_market.as_ref().unwrap().id), &[]).await?;

        response_into::<manifold_types::FullMarket>(full_market).await
    }

    /// Initializes a tx, rx pair for the bot. The tx channel is used by the
    /// bots send bets to the MarketHandler, and is many-to-one. The Reciever
    /// channel is used by the MarketHandler to send the responses, and is
    /// one-to-one. Each channel
    pub async fn posty_init(
        &mut self,
        bot_id: String,
    ) -> Result<(mpsc::Sender<PostyPacket>, broadcast::Receiver<PostyPacket>), String> {
        // if id is in hashmap, bail
        if self.bot_out_channel.lock().unwrap().contains_key(&bot_id) {
            return Err(format!("Bot {bot_id} already exists"));
        }

        let bot_to_mh_tx = self.bots_to_mh_tx.clone();

        let (tx_bot, rx_bot) = broadcast::channel::<PostyPacket>(4);
        self.bot_out_channel.lock().unwrap().insert(bot_id, tx_bot);

        Ok((bot_to_mh_tx, rx_bot))
    }

    pub async fn get_bet_stream_for_market_id(
        &mut self,
        market_id: String,
    ) -> broadcast::Receiver<manifold_types::Bet> {
        self.get_bet_stream(
            market_id.clone(),
            vec![("contractId".to_string(), market_id)],
        )
        .await
    }

    pub async fn get_bet_stream(
        &mut self,
        stream_key: String,
        query_params: Vec<(String, String)>,
    ) -> broadcast::Receiver<manifold_types::Bet> {
        info!(
            "Getting bet stream for {stream_key} params {:?}",
            query_params
        );

        let rx = if self.bet_channels.contains_key(&stream_key) {
            self.bet_channels[&stream_key].subscribe()
        } else {
            let (tx, rx) = broadcast::channel::<manifold_types::Bet>(128);
            self.bet_channels
                .entry(stream_key.to_string())
                .or_insert(tx);
            rx
        };

        let mut base_query = query_params.to_vec();
        base_query.push(("limit".to_string(), "1".to_string()));

        let response = get_endpoint("bets".to_string(), &base_query)
            .await
            .expect("Couldn't get most recent bet from api");

        let mut most_recent_id = response_into::<Vec<manifold_types::Bet>>(response)
            .await
            .expect("Couldn't convert json into Bet")
            .pop()
            .expect("no bets placed yet")
            .id;

        // Spawn the task that gets messages from the api and
        // sends them to the channel
        let tx_clone = self.bet_channels[&stream_key].clone();
        let halt_flag_clone = self.halt_flag.clone();

        tokio::spawn(async move {
            while !halt_flag_clone.load(Ordering::SeqCst) {
                let mut params = query_params.clone();
                params.push(("after".to_string(), most_recent_id.clone()));

                let resp = match get_endpoint("bets".to_string(), &params).await {
                    Ok(resp) => resp,
                    Err(e) => {
                        warn!("continuing... couldn't get most recent bet due to api error: {e}");
                        continue;
                    }
                };

                let bets = response_into::<Vec<manifold_types::Bet>>(resp)
                    .await
                    .expect("Couldn't convert json into Bet");

                for bet in bets.iter() {
                    tx_clone.send(bet.clone()).expect("Couldn't send bet");
                }

                if !bets.is_empty() {
                    most_recent_id = bets.last().unwrap().id.clone();
                }

                sleep(Duration::from_millis(500)).await;
            }
        });

        rx
    }
}

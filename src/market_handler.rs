use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{broadcast, mpsc};
use tokio::time::{sleep, Duration};

use crate::errors;
use crate::manifold_types;
use crate::rate_limiter;
use crate::utils;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Method {
    Get,
    Post,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InternalPacket {
    bot_id: String,
    method: Method,
    endpoint: String,
    query_params: Vec<(String, String)>,
    data: Option<Value>,
    response: Option<String>,
}

impl InternalPacket {
    pub fn new(
        bot_id: String,
        method: Method,
        endpoint: String,
        query_params: Vec<(String, String)>,
        data: Option<Value>,
    ) -> Self {
        Self {
            bot_id,
            method,
            endpoint,
            query_params,
            data,
            response: None,
        }
    }

    pub fn response_from_existing(packet: &InternalPacket, response: String) -> Self {
        Self {
            bot_id: packet.bot_id.clone(),
            method: packet.method.clone(),
            endpoint: packet.endpoint.clone(),
            query_params: packet.query_params.clone(),
            data: packet.data.clone(),
            response: Some(response),
        }
    }
}

async fn rate_limited_post_endpoint(
    mut write_rate_limiter: rate_limiter::RateLimiter,
    endpoint: String,
    query_params: &[(String, String)],
    data: Option<Value>,
) -> Result<reqwest::Response, reqwest::Error> {
    if write_rate_limiter.block_for_average_pace_then_commit(Duration::from_secs(60)) {
        utils::post_endpoint(endpoint, query_params, data).await
    } else {
        panic!(
            "rate limiter timed out; this shouldn't be possible, \
            most likely rate limit is set wrong"
        );
    }
}

async fn rate_limited_get_endpoint(
    mut read_rate_limiter: rate_limiter::RateLimiter,
    endpoint: String,
    query_params: &[(String, String)],
) -> Result<reqwest::Response, reqwest::Error> {
    if read_rate_limiter.block_for_average_pace_then_commit(Duration::from_secs(1)) {
        utils::get_endpoint(endpoint, query_params).await
    } else {
        panic!(
            "rate limiter timed out; this shouldn't be possible, \
            most likely rate limit is set wrong"
        );
    }
}

pub async fn send_internal_packet(
    read_rate_limiter: &rate_limiter::RateLimiter,
    write_rate_limiter: &rate_limiter::RateLimiter,
    internal_coms_packet: &InternalPacket,
) -> Result<reqwest::Response, reqwest::Error> {
    match internal_coms_packet.method {
        Method::Get => {
            rate_limited_get_endpoint(
                read_rate_limiter.clone(),
                internal_coms_packet.endpoint.clone(),
                &internal_coms_packet.query_params,
            )
            .await
        }
        Method::Post => {
            rate_limited_post_endpoint(
                write_rate_limiter.clone(),
                internal_coms_packet.endpoint.clone(),
                &internal_coms_packet.query_params,
                internal_coms_packet.data.clone(),
            )
            .await
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct MarketHandler {
    halt_flag: Arc<AtomicBool>,

    bots_to_mh_tx: mpsc::Sender<InternalPacket>,
    bot_out_channel: Arc<Mutex<HashMap<String, broadcast::Sender<InternalPacket>>>>,

    read_rate_limiter: rate_limiter::RateLimiter,
    write_rate_limiter: rate_limiter::RateLimiter,

    bet_channels: HashMap<String, broadcast::Sender<manifold_types::Bet>>,
}

#[allow(dead_code)]
impl MarketHandler {
    pub fn new() -> Self {
        let halt_flag = Arc::new(AtomicBool::new(false));

        let (bots_to_mh_tx, bots_to_mh_rx) = mpsc::channel::<InternalPacket>(256);
        let bot_out_channel: Arc<Mutex<HashMap<String, broadcast::Sender<InternalPacket>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        let halt_flag_clone = halt_flag.clone();
        let bot_out_channel_clone = bot_out_channel.clone();

        // set the rate limits slightly lower than the true value
        let read_rate_limiter = rate_limiter::RateLimiter::new(90, Duration::from_secs(1));
        let write_rate_limiter = rate_limiter::RateLimiter::new(9, Duration::from_secs(60));

        tokio::spawn(Self::handle_bot_messages(
            write_rate_limiter.clone(),
            read_rate_limiter.clone(),
            halt_flag_clone,
            bots_to_mh_rx,
            bot_out_channel_clone,
        ));

        Self {
            halt_flag,
            bots_to_mh_tx,
            bot_out_channel,
            read_rate_limiter,
            write_rate_limiter,
            bet_channels: HashMap::new(),
        }
    }

    pub fn send_to_bots(
        bot_out_channel: &Arc<Mutex<HashMap<String, broadcast::Sender<InternalPacket>>>>,
        bot_id: &String,
        packet: InternalPacket,
    ) {
        bot_out_channel
            .lock()
            .unwrap()
            .get(bot_id)
            .unwrap()
            .send(packet)
            .expect("couldn't send internal_coms packet");
    }

    async fn handle_bot_messages(
        write_rate_limiter: rate_limiter::RateLimiter,
        read_rate_limiter: rate_limiter::RateLimiter,
        halt_flag: Arc<AtomicBool>,
        mut bots_to_mh_rx: mpsc::Receiver<InternalPacket>,
        bot_out_channel: Arc<Mutex<HashMap<String, broadcast::Sender<InternalPacket>>>>,
    ) {
        while !halt_flag.load(Ordering::SeqCst) {
            // why a Option instead of a Result here?
            let internal_coms_packet = match bots_to_mh_rx.recv().await {
                Some(packet) => packet,
                None => continue,
            };

            debug!("got internal_coms packet {:?}", internal_coms_packet);

            let maybe_res = send_internal_packet(
                &read_rate_limiter,
                &write_rate_limiter,
                &internal_coms_packet,
            )
            .await;

            let res = match maybe_res {
                Ok(res) => res,
                Err(e) => {
                    error!("api error {e}");
                    let packet = InternalPacket::response_from_existing(
                        &internal_coms_packet,
                        format!("api error {e}"),
                    );

                    Self::send_to_bots(&bot_out_channel, &internal_coms_packet.bot_id, packet);

                    continue;
                }
            }
            .text()
            .await
            .unwrap();

            let packet = InternalPacket::response_from_existing(&internal_coms_packet, res);
            Self::send_to_bots(&bot_out_channel, &internal_coms_packet.bot_id, packet);
        }
    }

    pub fn halt(&self) {
        self.halt_flag.store(true, Ordering::SeqCst);
    }

    pub async fn check_alive(&self) -> bool {
        let resp = rate_limited_get_endpoint(self.read_rate_limiter.clone(), "me".to_string(), &[])
            .await
            .unwrap();

        resp.json::<manifold_types::User>().await.is_ok()
    }

    pub async fn whoami(&self) -> Result<manifold_types::User, reqwest::Error> {
        let resp = rate_limited_get_endpoint(self.read_rate_limiter.clone(), "me".to_string(), &[])
            .await
            .unwrap();

        resp.json::<manifold_types::User>().await
    }

    pub async fn get_all_my_positions(
        &self,
    ) -> Result<Vec<manifold_types::Bet>, errors::ReqwestResponseParsing> {
        let me = match self.whoami().await {
            Ok(me) => me,
            Err(e) => {
                error!("couldn't get me: {e}");
                return Err(format!("couldn't get me: {e}").into());
            }
        };

        let mut bet_before_id: String = "".to_string();
        let mut all_bets: Vec<manifold_types::Bet> = vec![];

        loop {
            let params = [
                ("userId".to_string(), me.id.clone()),
                ("before".to_string(), bet_before_id),
                ("limit".to_string(), "1000".to_string()),
            ];

            let bets_response = rate_limited_get_endpoint(
                self.read_rate_limiter.clone(),
                "bets".to_string(),
                &params,
            )
            .await;

            let bets = match bets_response {
                Ok(bets_response) => bets_response.json::<Vec<manifold_types::Bet>>().await?,
                Err(e) => {
                    error!("couldn't get bets: {e}");
                    return Err(format!("couldn't get bets: {e}").into());
                }
            };

            all_bets.extend(bets.clone());

            if bets.len() < 1000 {
                break;
            } else {
                debug!("found {} bets", bets.len());
                bet_before_id = bets.last().unwrap().id.clone();
            }
        }
        Ok(all_bets)
    }

    pub async fn get_active_positions(
        all_bets: Vec<manifold_types::Bet>,
    ) -> Vec<manifold_types::Position> {
        #[derive(Hash, Eq, PartialEq)]
        struct PositionKey {
            outcome: String,
            contract_id: String,
            answer_id: Option<String>,
        }

        let mut all_positions: HashMap<PositionKey, f64> = HashMap::new();

        for bet in all_bets {
            let position = PositionKey {
                outcome: bet.outcome,
                contract_id: bet.contract_id,
                answer_id: bet.answer_id,
            };
            *all_positions.entry(position).or_insert(0.0) += bet.amount;
        }

        all_positions
            .into_iter()
            .filter(|(_, total_amount_sum)| *total_amount_sum > 1e-10)
            .map(|(position, total_amount_sum)| manifold_types::Position {
                outcome: position.outcome,
                contract_id: position.contract_id,
                answer_id: position.answer_id,
                amount: total_amount_sum,
            })
            .collect::<Vec<manifold_types::Position>>()
    }

    pub async fn liquidate_all_positions(&self) -> Result<(), errors::ReqwestResponseParsing> {
        let all_bets = self.get_all_my_positions().await?;
        let open_positions = Self::get_active_positions(all_bets.clone()).await;

        for pos in open_positions {
            let data = Some(serde_json::json!({
                "answerId": pos.answer_id,
            }));

            let sell_response = rate_limited_post_endpoint(
                self.write_rate_limiter.clone(),
                format!("market/{}/sell", pos.contract_id),
                &[],
                data,
            )
            .await;

            match sell_response {
                Ok(resp) => {
                    info!(
                        "successfully sold {} {} shares for contract id {} answer id {:?}",
                        pos.amount, pos.outcome, pos.contract_id, pos.answer_id
                    );
                    debug!(
                        "full response {:?} for contract {} answer {:?}",
                        resp, pos.contract_id, pos.answer_id
                    );
                }
                Err(e) => error!("couldn't sell shares: {e}"),
            };
        }

        Ok(())
    }

    pub async fn market_search(
        &self,
        term: String,
    ) -> Result<manifold_types::FullMarket, errors::ReqwestResponseParsing> {
        let resp = rate_limited_get_endpoint(
            self.read_rate_limiter.clone(),
            "search-markets".to_string(),
            &[
                ("term".to_string(), term.clone()),
                ("limit".to_string(), "1".to_string()),
            ],
        )
        .await
        .unwrap();

        let lite_market_req = utils::response_into::<Vec<manifold_types::LiteMarket>>(resp).await;
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

        let full_market = rate_limited_get_endpoint(
            self.read_rate_limiter.clone(),
            format!("market/{}", lite_market.as_ref().unwrap().id),
            &[],
        )
        .await
        .unwrap();

        utils::response_into::<manifold_types::FullMarket>(full_market).await
    }

    /// Initializes a tx, rx pair for the bot. The tx channel is used by the
    /// bots send bets to the MarketHandler, and is many-to-one. The Reciever
    /// channel is used by the MarketHandler to send the responses, and is
    /// one-to-one. Each channel
    pub async fn internal_coms_init(
        &mut self,
        bot_id: String,
    ) -> Result<
        (
            mpsc::Sender<InternalPacket>,
            broadcast::Receiver<InternalPacket>,
        ),
        String,
    > {
        // if id is in hashmap, bail
        if self.bot_out_channel.lock().unwrap().contains_key(&bot_id) {
            return Err(format!("Bot {bot_id} already exists"));
        }

        let bot_to_mh_tx = self.bots_to_mh_tx.clone();

        let (tx_bot, rx_bot) = broadcast::channel::<InternalPacket>(4);
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

        let response = rate_limited_get_endpoint(
            self.read_rate_limiter.clone(),
            "bets".to_string(),
            &base_query,
        )
        .await
        .expect("Couldn't get most recent bet from api");

        let mut most_recent_id = utils::response_into::<Vec<manifold_types::Bet>>(response)
            .await
            .expect("Couldn't convert json into Bet")
            .pop()
            .expect("no bets placed yet")
            .id;

        // Spawn the task that gets messages from the api and
        // sends them to the channel
        let tx_clone = self.bet_channels[&stream_key].clone();
        let halt_flag_clone = self.halt_flag.clone();
        let mut read_rate_limiter_clone = self.read_rate_limiter.clone();

        tokio::spawn(async move {
            while !halt_flag_clone.load(Ordering::SeqCst) {
                let mut params = query_params.clone();
                params.push(("after".to_string(), most_recent_id.clone()));

                let committed = read_rate_limiter_clone
                    .block_for_average_pace_then_commit(Duration::from_millis(500));

                if !committed {
                    warn!("continuing... couldn't get most recent bet due to rate limit - we timed out");
                    continue;
                }

                let resp = match rate_limited_get_endpoint(
                    read_rate_limiter_clone.clone(),
                    "bets".to_string(),
                    &params,
                )
                .await
                {
                    Ok(resp) => resp,
                    Err(e) => {
                        warn!("continuing... couldn't get most recent bet due to api error: {e}");
                        continue;
                    }
                };

                let bets = utils::response_into::<Vec<manifold_types::Bet>>(resp)
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

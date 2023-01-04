use std::env;
use std::{collections::HashMap, time::Duration};

use dotenv::dotenv;
use reqwest::{
    self,
    header::{self, HeaderMap},
};
use reqwest::{Client, Proxy};
use serde::Deserialize;
use tokio::time;
use webhook::client::WebhookClient;

#[derive(Deserialize, Debug)]
struct Response {
    #[serde(rename = "Product")]
    product: Product,
}

#[derive(Deserialize, Debug)]
struct Product {
    children: HashMap<String, ProductData>,
}

#[derive(Deserialize, Debug)]
struct ProductData {
    #[serde(rename = "shoeSize")]
    shoe_size: String,
    market: Market,
}

#[derive(Deserialize, Debug)]
struct Market {
    #[serde(rename = "highestBid")]
    highest_bid: u16,
}

async fn send_webhook(url: &String, ask_price: u16) {
    let client: WebhookClient = WebhookClient::new(url);
    client
        .send(|message| message.content(&format!("New ask! {}£ @everyone", ask_price)))
        .await
        .expect("Could not send a message to a webhook");
}

fn create_http_client() -> Client {
    let proxy_url = env::var("PROXY").expect("Proxy url is empty");
    let proxy_user = env::var("PROXY_USER").expect("Proxy user is empty");
    let proxy_pwd = env::var("PROXY_PWD").expect("Proxy pwd is empty");

    let proxy_ob: Proxy = reqwest::Proxy::all(proxy_url)
        .unwrap()
        .basic_auth(&proxy_user, &proxy_pwd);

    let mut headers: HeaderMap = header::HeaderMap::new();
    headers.insert(
        "User-Agent",
        header::HeaderValue::from_static("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.1 Safari/605.1.15")
    );
    headers.insert(
        "Content-Type",
        header::HeaderValue::from_static("application/json"),
    );

    reqwest::ClientBuilder::new()
        .default_headers(headers)
        .proxy(proxy_ob)
        .build()
        .unwrap()
}

async fn get_stockx_data(client: &Client) -> Result<Response, reqwest::Error> {
    client.get("https://stockx.com/api/products/adidas-samba-black-white-gum/?currency=GBP&includes=market")
        .send()
        .await?
        .json::<Response>()
        .await
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let webook_url = env::var("WEBHOOK_URL").expect("Webhook url is empty");
    let client = create_http_client();

    let mut last_price: u16 = 0;
    loop {
        let req: Result<Response, reqwest::Error> = get_stockx_data(&client).await;
        let resp = match req {
            Ok(resp) => resp,
            Err(error) => panic!("{:?}", error),
        };

        let items: HashMap<String, ProductData> = resp.product.children;
        for item in items {
            let bid: u16 = item.1.market.highest_bid;
            if item.1.shoe_size == "6" && last_price < bid {
                last_price = bid;
                send_webhook(&webook_url, bid).await;
            }
        }
        time::sleep(Duration::from_secs(300)).await;
    }
}

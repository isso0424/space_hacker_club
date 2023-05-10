use chrono::Utc;
use colored::Colorize;
use http::StatusCode;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct Item {
    symbol: String,
    units: i8,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Default, Clone)]
struct Cargo {
    capacity: i8,
    units: i8,
    inventory: Vec<Item>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct ShipInfoResponse {
    cargo: Cargo,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct Response<T> {
    data: T,
}

#[derive(serde::Serialize, Debug)]
struct SellRequest<'a> {
    symbol: &'a str,
    units: &'a i8,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Cooldown {
    ship_symbol: String,
    total_seconds: u8,
    remaining_seconds: u8,
    expiration: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct ExtractionYield {
    symbol: String,
    units: u8,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Extraction {
    r#yield: ExtractionYield,
    ship_symbol: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Agent {
    account_id: String,
    symbol: String,
    headquarters: String,
    credits: u32,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Transaction {
    waypoint_symbol: String,
    ship_symbol: String,
    trade_symbol: String,
    r#type: String,
    units: u8,
    price_per_unit: u16,
    total_price: u16,
    timestamp: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct ExtractResponse {
    cooldown: Cooldown,
    extraction: Extraction,
    cargo: Cargo,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct ConflictError {
    cooldown: Cooldown,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct Error<T> {
    error: Response<T>,
}

#[derive(Debug)]
enum LogType {
    Extract,
    Sell,
    Navigate,
    Deliver,
    Dock,
}

impl std::fmt::Display for LogType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogType::Deliver => write!(f, "DELIVER"),
            LogType::Sell => write!(f, "SELL"),
            LogType::Dock => write!(f, "DOCK"),
            LogType::Navigate => write!(f, "NAVIGATE"),
            LogType::Extract => write!(f, "EXTRACT"),
        }
    }
}

impl LogType {
    fn colored(&self) -> colored::ColoredString {
        match self {
            LogType::Deliver => "DELIVER".on_red(),
            LogType::Sell => "SELL".on_green(),
            LogType::Dock => "DOCK".on_yellow(),
            LogType::Navigate => "NAVIGATE".on_blue(),
            LogType::Extract => "EXTRACT".on_magenta(),
        }
    }
}

fn color_with_number(text: &str, num: i32) -> colored::ColoredString {
    match num % 7 {
        0 => text.white(),
        1 => text.red(),
        2 => text.green(),
        3 => text.yellow(),
        4 => text.blue(),
        5 => text.magenta(),
        6 => text.cyan(),
        _ => text.red(),
    }
}

fn log(ship_name: &str, message: &str, log_type: LogType) {
    let (_, num) = ship_name.clone().split_once("-").unwrap();

    println!(
        "{}{} {}",
        color_with_number(ship_name, num.to_string().parse::<i32>().unwrap()).bold(),
        log_type.colored().bold(),
        message
    )
}

async fn extract(client: &reqwest::Client, ship_name: &str) -> Result<(), reqwest::Error> {
    loop {
        let res = client
            .post(format!(
                "https://api.spacetraders.io/v2/my/ships/{}/extract",
                ship_name,
            ))
            .header(reqwest::header::CONTENT_LENGTH, 0)
            .body("")
            .send()
            .await?;

        if res.status() == StatusCode::BAD_REQUEST {
            break;
        }

        match res.status() {
            StatusCode::CREATED => {
                let response: Response<ExtractResponse> = res.json().await.unwrap();
                log(
                    ship_name,
                    &format!(
                        "extract succeed (material: {} amount: {})",
                        response.data.extraction.r#yield.symbol,
                        response.data.extraction.r#yield.units
                    ),
                    LogType::Extract,
                );

                if response.data.cargo.units == response.data.cargo.capacity {
                    log(ship_name, "extract completed", LogType::Extract);
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(
                    response.data.cooldown.remaining_seconds as u64,
                ))
                .await;
            }
            StatusCode::CONFLICT => {
                let response: Error<ConflictError> = res.json().await.unwrap();
                log(
                    ship_name,
                    &format!(
                        "cooldown exceeded (remaining {} secs)",
                        response.error.data.cooldown.remaining_seconds
                    ),
                    LogType::Extract,
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(
                    response.error.data.cooldown.remaining_seconds as u64,
                ))
                .await;
            }
            StatusCode::BAD_REQUEST => {
                break;
            }
            _ => {
                log(
                    ship_name,
                    &format!("error occured in extraction ({})", res.status()),
                    LogType::Extract,
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(75)).await;
            }
        }
    }
    Ok(())
}

async fn fetch_cargo_status(
    client: &reqwest::Client,
    ship_name: &str,
) -> Result<Cargo, reqwest::Error> {
    let res = client
        .get(format!(
            "https://api.spacetraders.io/v2/my/ships/{}",
            ship_name,
        ))
        .header(reqwest::header::CONTENT_LENGTH, 0)
        .body("")
        .send()
        .await?;

    let data: Response<ShipInfoResponse> = res.json().await?;

    Ok(data.data.cargo)
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct SellItemResponse {
    cargo: Cargo,
    transaction: Transaction,
    agent: Agent,
}

async fn sell_item(
    client: &reqwest::Client,
    ship_name: &str,
    item: &Item,
) -> Result<(), reqwest::Error> {
    if item.symbol == "ANTIMATTER" {
        return Ok(());
    }
    if item.symbol == "ALUMINUM_ORE" {
        log(
            ship_name,
            &format!("sold skipped ({})", item.symbol),
            LogType::Sell,
        );
        return Ok(());
    }
    let req = SellRequest {
        symbol: &item.symbol,
        units: &item.units,
    };

    let res = client
        .post(format!(
            "https://api.spacetraders.io/v2/my/ships/{}/sell",
            ship_name,
        ))
        .json(&req)
        .send()
        .await
        .unwrap();
    if res.status() == StatusCode::CREATED {
        let j: Response<SellItemResponse> = res.json().await.unwrap();
        log(
            ship_name,
            &format!(
                "sold succeed (material: {} unit: {} currentCredits: {}(+{}))",
                j.data.transaction.trade_symbol,
                j.data.transaction.units,
                j.data.agent.credits,
                j.data.transaction.total_price,
            ),
            LogType::Sell,
        );
    } else {
        log(
            ship_name,
            &format!("error occured in selling ({})", res.status()),
            LogType::Sell,
        );
    }

    Ok(())
}

#[derive(serde::Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct NavigateRequest<'a> {
    waypoint_symbol: &'a str,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct WayPoint {
    symbol: String,
    system_symbol: String,
    x: i8,
    y: i8,
}

#[derive(serde::Deserialize, Debug)]
struct Route {
    destination: WayPoint,
    departure: WayPoint,
    arrival: String,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Nav {
    system_symbol: String,
    waypoint_symbol: String,
    route: Route,
}

#[derive(serde::Deserialize, Debug)]
struct NavigateResponse {
    nav: Nav,
}

async fn refuel(client: &reqwest::Client, ship_name: &str) -> Result<(), reqwest::Error> {
    dock(&client, ship_name).await?;
    client
        .post(format!(
            "https://api.spacetraders.io/v2/my/ships/{}/refuel",
            ship_name,
        ))
        .header(reqwest::header::CONTENT_LENGTH, 0)
        .send()
        .await?;

    Ok(())
}

async fn navigate(
    client: &reqwest::Client,
    ship_name: &str,
    target: &str,
) -> Result<Nav, reqwest::Error> {
    refuel(client, ship_name).await?;
    let req = NavigateRequest {
        waypoint_symbol: target,
    };
    let res = client
        .post(format!(
            "https://api.spacetraders.io/v2/my/ships/{}/navigate",
            ship_name,
        ))
        .json(&req)
        .send()
        .await?;

    let r: Response<NavigateResponse> = res.json().await?;

    let now = Utc::now();
    let raw_arrival = chrono::DateTime::parse_from_rfc3339(&r.data.nav.route.arrival).unwrap();
    let arrival = raw_arrival.with_timezone(&chrono::Utc);
    let duration = arrival - now;

    log(
        ship_name,
        &format!(
            "navigate {} -> {}",
            r.data.nav.route.departure.symbol, r.data.nav.route.destination.symbol
        ),
        LogType::Navigate,
    );
    tokio::time::sleep(tokio::time::Duration::from_secs(
        duration.num_seconds().try_into().unwrap(),
    ))
    .await;

    Ok(r.data.nav)
}

async fn dock(client: &reqwest::Client, ship_name: &str) -> Result<(), reqwest::Error> {
    client
        .post(format!(
            "https://api.spacetraders.io/v2/my/ships/{}/dock",
            ship_name,
        ))
        .header(reqwest::header::CONTENT_LENGTH, 0)
        .send()
        .await?;

    log(ship_name, "docked", LogType::Dock);

    Ok(())
}

#[derive(serde::Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DeliverRequest<'a> {
    ship_symbol: &'a str,
    trade_symbol: &'a str,
    units: &'a i8,
}

async fn deliver(
    client: &reqwest::Client,
    ship_name: &str,
    target: &str,
    contract_id: &str,
    item: &Item,
) -> Result<(), reqwest::Error> {
    log(
        ship_name,
        &format!("deliver {} to {}", item.symbol, target),
        LogType::Deliver,
    );
    let nav = navigate(&client, &ship_name, &target).await?;

    dock(&client, ship_name).await?;
    let req = DeliverRequest {
        ship_symbol: ship_name,
        trade_symbol: &item.symbol,
        units: &item.units,
    };
    client
        .post(format!(
            "https://api.spacetraders.io/v2/my/contracts/{}/deliver",
            contract_id,
        ))
        .json(&req)
        .send()
        .await?;
    log(
        ship_name,
        &format!("delivered {}", item.symbol),
        LogType::Deliver,
    );

    navigate(&client, &ship_name, &nav.route.departure.symbol).await?;
    dock(&client, ship_name).await?;

    log(ship_name, "deliver completed", LogType::Deliver);

    Ok(())
}

fn check_contract_material(ship_name: &str, cargo: &Cargo, material: &str) -> Option<Item> {
    let target = cargo
        .inventory
        .clone()
        .into_iter()
        .find(|item| item.symbol == material);
    if let Some(item) = target {
        let percentage = item.units as f32 / cargo.capacity as f32 * 100.0;

        log(
            ship_name,
            &format!(
                "progress to deliver {:3.1}% ({}/{})",
                percentage, item.units, cargo.capacity
            ),
            LogType::Deliver,
        );

        if item.units as f32 / cargo.capacity as f32 >= 0.75 {
            Some(item)
        } else {
            None
        }
    } else {
        None
    }
}

async fn loop_selling(client: &reqwest::Client, ship_name: &str) -> Result<(), reqwest::Error> {
    loop {
        extract(&client, ship_name).await?;

        let cargo = fetch_cargo_status(&client, ship_name).await?;

        log(ship_name, "sell items", LogType::Sell);

        let mut iter = cargo.inventory.clone().into_iter();
        if let Some(item) = check_contract_material(ship_name, &cargo, "ALUMINUM_ORE") {
            deliver(
                &client,
                ship_name,
                "X1-DF55-20250Z",
                "clhfmst0d00xjs60d7xs2vg8p",
                &item,
            )
            .await?;
        }
        for _ in 0..cargo.inventory.len() {
            let item = iter.nth(0);
            if let Some(item) = item {
                sell_item(&client, ship_name, &item).await?;
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
        log(ship_name, "sell completed", LogType::Sell);
    }
}

async fn build_client() -> Result<reqwest::Client, Box<dyn std::error::Error + Send + Sync>> {
    let mut headers = reqwest::header::HeaderMap::new();

    let token = format!("Bearer {}", std::env::var("API_TOKEN")?);

    headers.insert(
        reqwest::header::AUTHORIZATION,
        reqwest::header::HeaderValue::from_str(&token)?,
    );

    return Ok(reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap());
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenvy::dotenv()?;

    let threads: Vec<_> = vec!["SOIES-1", "SOIES-2", "SOIES-3", "SOIES-4"]
        .into_iter()
        .enumerate()
        .map(|(i, ship_name)| {
            tokio::spawn(async move {
                let client = build_client().await.unwrap();
                tokio::time::sleep(tokio::time::Duration::from_secs(i as u64 * 10)).await;
                loop_selling(&client, ship_name).await.unwrap();
            })
        })
        .collect();

    for handle in threads {
        handle.await.unwrap();
    }

    Ok(())
}

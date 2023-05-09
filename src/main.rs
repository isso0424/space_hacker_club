use chrono::Utc;
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
                println!(
                    "{}: extract succeed (material: {} amount: {})",
                    ship_name,
                    response.data.extraction.r#yield.symbol,
                    response.data.extraction.r#yield.units
                );

                if response.data.cargo.units == response.data.cargo.capacity {
                    println!("{}: extract completed", ship_name);
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(
                    response.data.cooldown.remaining_seconds as u64,
                ))
                .await;
            }
            StatusCode::CONFLICT => {
                let response: Error<ConflictError> = res.json().await.unwrap();
                println!(
                    "{}: cooldown exceeded (remaining {} secs)",
                    ship_name, response.error.data.cooldown.remaining_seconds
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
                println!(
                    "{}: error occured in extraction ({})",
                    ship_name,
                    res.status()
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
        println!("{}: sold skipped ({})", ship_name, item.symbol);
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
        println!(
            "{}: sold succeed (material: {} unit: {} currentCredits: {}(+{}))",
            ship_name,
            j.data.transaction.trade_symbol,
            j.data.transaction.units,
            j.data.agent.credits,
            j.data.transaction.total_price,
        );
    } else {
        println!("{}: error occured in selling ({})", ship_name, res.status());
    }

    Ok(())
}

#[derive(serde::Serialize, Debug)]
struct NavigateRequest<'a> {
    waypoint_symbol: &'a str,
}

#[derive(serde::Deserialize, Debug)]
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
    client
        .post(format!(
            "https://api.spacetraders.io/v2/my/ships/{}/refuel",
            ship_name,
        ))
        .json("")
        .send()
        .await?;

    Ok(())
}

async fn navigate(
    client: &reqwest::Client,
    ship_name: &str,
    target: &str,
) -> Result<Nav, reqwest::Error> {
    let req = NavigateRequest {
        waypoint_symbol: target,
    };
    refuel(client, ship_name).await?;
    let res = client
        .post(format!(
            "https://api.spacetraders.io/v2/my/ships/{}/navigate",
            ship_name,
        ))
        .json(&req)
        .send()
        .await?;

    let r: Response<NavigateResponse> = res.json().await?;
    println!("{:?}", r);

    let now = Utc::now();
    let raw_arrival = chrono::DateTime::parse_from_rfc3339(&r.data.nav.route.arrival).unwrap();
    let arrival = raw_arrival.with_timezone(&chrono::Utc);
    let duration = arrival - now;

    println!(
        "{}: navigate {} -> {}",
        ship_name, r.data.nav.route.departure.symbol, r.data.nav.route.destination.symbol
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
        .json("")
        .send()
        .await?;

    println!("{}: docked", ship_name);

    Ok(())
}

#[derive(serde::Serialize, Debug)]
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
    let nav = navigate(&client, &ship_name, &target).await?;

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

    navigate(&client, &ship_name, &nav.route.departure.symbol).await?;
    dock(&client, ship_name).await?;

    Ok(())
}

fn check_contract_material(cargo: &Cargo, material: &str) -> Option<Item> {
    let target = cargo
        .inventory
        .clone()
        .into_iter()
        .find(|item| item.symbol == material);
    if let Some(item) = target {
        if item.units as f32 / cargo.capacity as f32 > 0.6 {
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
        println!("{}: fetch completed", ship_name);

        println!("{}: sell items", ship_name);

        let mut iter = cargo.inventory.clone().into_iter();
        if let Some(item) = check_contract_material(&cargo, "ALUMINUM_ORE") {
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
        println!("{}: sell completed", ship_name);
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

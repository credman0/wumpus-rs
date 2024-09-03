use reqwest::{Client, header};
use structopt::StructOpt;
use csv::ReaderBuilder;
use std::fs::File;
use std::fs;
use std::io::Write;
use std::path::Path;
use dirs::home_dir;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(long, default_value = "my_cube")]
    cube_name: String,

    #[structopt(long)]
    without_rarity: bool,

    #[structopt(long, default_value = "akh,dom,war,stx,znr")]
    sets: String,
}

async fn fetch_page(client: &Client, url: &str) -> Result<(reqwest::Response, Option<String>), reqwest::Error> {
    let response = client.get(url).send().await?;
    let next_page_url = response.headers().get("X-Scryfall-Next-Page")
        .map(|v| v.to_str().unwrap_or("").to_string());
    Ok((response, next_page_url))
}

fn process_csv_text(text: &str, with_rarity: bool) -> Vec<Vec<String>> {
    let mut names = Vec::new();
    let csv_data = text.lines().skip(1);
    let csv_data = csv_data.collect::<Vec<&str>>().join("\n");
    let mut rdr = ReaderBuilder::new().has_headers(false).from_reader(csv_data.as_bytes());

    for result in rdr.records() {
        let record = result.expect("Failed to read record");
        if with_rarity {
            names.push(vec![record[6].to_string(), record[5].to_string()]);
        } else {
            names.push(vec![record[6].to_string()]);
        }
    }
    names
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();

    // Set up headers
    let mut headers = header::HeaderMap::new();
    headers.insert(header::USER_AGENT, header::HeaderValue::from_static("YourAppName/0.1"));
    headers.insert(header::ACCEPT, header::HeaderValue::from_static("application/json;q=0.9,*/*;q=0.8"));

    // Create client with headers and TLS settings
    let client = Client::builder()
        .default_headers(headers)
        .build()?;

    let sets: Vec<&str> = opt.sets.split(',').collect();
    let query = format!("-t%3ABasic+AND+game%3Apaper+AND+({})",
        sets.iter().map(|s| format!("set%3A{}", s)).collect::<Vec<_>>().join("+OR+"));
    let base_url = format!("https://api.scryfall.com/cards/search?order=name&format=csv&q=({})&page=1", query);

    let mut cube = Vec::new();
    let mut url = base_url;

    while let Ok((response, next_page_url)) = fetch_page(&client, &url).await {
        if response.status().is_success() {
            let text = response.text().await?;
            let with_rarity = !opt.without_rarity;
            cube.extend(process_csv_text(&text, with_rarity));
            
            if let Some(next_page) = next_page_url {
                url = next_page;
            } else {
                break;
            }
        } else {
            eprintln!("URL: {}", url);
            eprintln!("Status: {}", response.status());
            eprintln!("Request failed");
            std::process::exit(1);
        }
    }

    let home_dir = home_dir().ok_or("Could not determine home directory")?;
    let save_dir = home_dir.join("Downloads");

    if opt.cube_name.is_empty() {
        for card in cube {
            if !opt.without_rarity {
                println!("{}:{}", card[0], card[1]);
            } else {
                println!("{}", card[0]);
            }
        }
    } else {
        let mut idx = 1;
        let mut filename = save_dir.join(format!("{}{}.txt", opt.cube_name, idx));
        while filename.exists() {
            idx += 1;
            filename = save_dir.join(format!("{}{}.txt", opt.cube_name, idx));
        }

        let mut file = File::create(&filename)?;
        for card in cube {
            if !opt.without_rarity {
                writeln!(file, "{}:{}", card[0], card[1])?;
            } else {
                writeln!(file, "{}", card[0])?;
            }
        }
        println!("Saved to {}", filename.display());
    }

    Ok(())
}

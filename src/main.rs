#![feature(let_chains, array_chunks)]
//!
//! z0rdl z0r archiver
//! made by unknowntrojan
//!
//! please don't hammer the servers too hard.
//! to be polite, increase POOL_SIZE to 1000.
//!

/// this specifies how many entries get processed per thread.
///
/// a thread is spawned for every `POOL_SIZE` entries, and downloads them in sequence.
///
/// a lower value means more threads, but I don't recommend going below 500.
const POOL_SIZE: usize = 500;

async fn download_id(id: usize) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("https://z0r.de/L/z0r-de_{id}.swf");

    let bytes = reqwest::get(url).await?.bytes().await?;

    tokio::fs::write(format!("z0r/{id}.swf"), bytes).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Attempting to download z0r!");

    // Get the amount of z0r entries
    let entries = {
        let page = reqwest::get("https://z0r.de/0").await?.text().await?;

        let parsed = tl::parse(page.as_str(), tl::ParserOptions::default())?;

        let previous_tag = parsed
            .nodes()
            .iter()
            .find(|x| x.inner_text(parsed.parser()) == "&laquo; Previous")
            .ok_or("Unable to find required HTML tag")?;

        let last_entry = previous_tag
            .as_tag()
            .ok_or("HTML tag was of wrong type")?
            .attributes()
            .get("href")
            .flatten()
            .ok_or("Unable to get href attribute from HTML tag")?;

        str::parse::<usize>(last_entry.as_utf8_str().to_string().as_str())?
    };

    println!("Z0R entries: {entries}");

    // Check which still need to be downloaded to disk
    let needs_download = {
        // this could be simpler and less cluttered, but we need array_chunks to work without skipping remainders
        let mut needs_download = vec![true; entries + (POOL_SIZE - entries % POOL_SIZE)];

        // hacky way to make array_chunks work for the remainder
        (entries..needs_download.len()).for_each(|i| {
            needs_download[i] = false;
        });

        // Create folder if doesn't exist, ignore error
        let _ = std::fs::create_dir("z0r");

        let folder = std::fs::read_dir("z0r")?;

        for entry in folder.flatten() {
            if let Some(filename) = entry.file_name().to_string_lossy().strip_suffix(".swf") {
                if let Ok(id) = str::parse::<usize>(filename) {
                    needs_download[id] = false;
                }
            }
        }

        needs_download
    };

    // Download!
    let tasks: Vec<_> = needs_download
        .array_chunks::<POOL_SIZE>()
        .enumerate()
        .map(|(page_id, needs_dl)| {
            let page_id = page_id;
            let needs_dl = *needs_dl;

            tokio::spawn(async move {
                let start_id = page_id * POOL_SIZE;

                println!("Downloading #{}-{}!", start_id, start_id + POOL_SIZE);

                for (id, needs_dl) in needs_dl.iter().enumerate() {
                    let z0r_id = id + start_id;
                    if *needs_dl && let Err(_) = download_id(z0r_id).await {
                        println!("Unable to download #{z0r_id}!");
                    } else if *needs_dl {
						println!("Successfully downloaded #{z0r_id}!");
					}
                }
            })
        })
        .collect();

    futures::future::join_all(tasks).await;

    println!("Successfully downloaded z0r!");

    Ok(())
}

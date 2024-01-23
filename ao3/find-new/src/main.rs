use std::time::Duration;

use amqprs::{
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::{BasicPublishArguments, QueueBindArguments},
    connection::{Connection, OpenConnectionArguments},
    BasicProperties,
};
use data_types::{NewWorkJob, USER_AGENT};
use rand::Rng;
use reqwest::{Client, Proxy};

#[tokio::main]
async fn main() {
    println!("Started!");
    tracing_subscriber::fmt::init();

    let proxy_url = std::env::var("PROXY_URL").expect("PROXY_URL must be provided");

    let client = reqwest::ClientBuilder::new()
        .proxy(Proxy::all(&proxy_url).expect("Failed to create proxy from URL"))
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(5))
        .build()
        .expect("Failed to build Reqwest client");

    let connection = Connection::open(&OpenConnectionArguments::new(
        &std::env::var("AMQP_SERVER").expect("AMQP_SERVER should be provided"),
        5672,
        &std::env::var("AMQP_USER").expect("AMQP_USER should be provided"),
        &std::env::var("AMQP_PASSWORD").expect("AMQP_PASSWORD should be provided"),
    ))
    .await
    .expect("Failed to connect to AMQP");
    println!("Connection established!");

    connection
        .register_callback(DefaultConnectionCallback)
        .await
        .unwrap();

    let works = tokio::spawn(run_new_works_loop(connection.clone(), client.clone()));

    tokio::select! {
        why = works => {println!("New works loop exited first: {why:?}"); return;}
        _ = tokio::signal::ctrl_c() => {println!("Exiting from signal"); return;}
    }
}

async fn run_new_works_loop(connection: Connection, client: Client) {
    let channel = connection
        .open_channel(None)
        .await
        .expect("Failed to open channel");

    channel
        .register_callback(DefaultChannelCallback)
        .await
        .unwrap();

    println!("New comment channel created!");

    channel
        .queue_bind(QueueBindArguments::new(
            "new-works",
            "amq.direct",
            "new-works",
        ))
        .await
        .unwrap();

    println!("Performed binding for new work channel");

    let mut last_final_id = 0; // this is the ID of the latest item on the last attempt
    let mut delay_secs = 30.0;

    let mut error_count = 0;
    let mut count_error = || {
        error_count += 1;
        tracing::error!("Error detected in new loop (current count: {error_count})");
        if error_count > 5 {
            panic!("Too many errors in new loop!")
        }
    };

    loop {
        let duration = rand::thread_rng().gen_range(delay_secs - 5.0..delay_secs + 5.0);
        let duration = std::time::Duration::from_secs_f32(duration);
        tracing::debug!("Sleeping for {duration:?}");
        tokio::time::sleep(duration).await;

        // Perform a GET to the web UI
        let request = client
            .get("https://archiveofourown.org/works/search?work_search[sort_column]=revised_at&work_search[sort_direction]=desc")
            .send()
            .await;
        let resp = match request {
            Ok(v) => v,
            Err(e) => {
                tracing::error!("Error while retrieving new works: {e}");
                count_error();
                continue;
            }
        };

        let text = match resp.text().await {
            Ok(v) => v,
            Err(e) => {
                tracing::error!("Error while parsing HTML page as text: {e}");
                count_error();
                continue;
            }
        };

        // Find all text which is like "/works/:id"
        let re = regex::Regex::new("href=\"/works/([0-9]+)\"").unwrap();
        let mut results = vec![];

        for (_, [id]) in re.captures_iter(&text).map(|c| c.extract()) {
            results.push(id.parse::<u64>().unwrap());
        }

        if results.is_empty() {
            tracing::error!("No works found on search page -- is this an error page?");
            count_error();
            continue;
        }
        let mut published = 0;

        for work_id in results.iter() {
            let work_id = *work_id;

            // If this work ID matches the previous attempt's final one, then we've caught up
            if work_id == last_final_id {
                break;
            }

            // Send this ID into the channel
            tracing::debug!("Found ID: {work_id}");

            let record = NewWorkJob::new(work_id);

            let json_data = serde_json::to_vec(&record);
            let json_data = match json_data {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!("Error while encoding work job to be sent: {e}");
                    count_error();
                    continue;
                }
            };
            channel
                .basic_publish(
                    BasicProperties::default(),
                    json_data,
                    BasicPublishArguments {
                        exchange: "amq.direct".to_string(),
                        routing_key: "new-works".to_string(),
                        mandatory: true,
                        immediate: false,
                    },
                )
                .await
                .unwrap();
            published += 1;
        }

        last_final_id = *results.first().unwrap();
        tracing::debug!("Published {published} new works");

        // Update the delay:
        // if zero works were received, increase the time (up to a maximum of 1800 seconds),
        // if more than one work was received, decrease the time (down to a minimum of 30 seconds).
        if published == 0 {
            delay_secs += 5.0;
        }
        if published > 1 {
            delay_secs -= 5.0;
        }
        delay_secs = delay_secs.min(1800.0).max(30.0);
    }
}

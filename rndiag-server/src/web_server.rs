use rndiag_core::tool::LatencyTool;
use warp::{http::StatusCode, reply::Reply, Filter};
use std::{convert::Infallible, sync::Arc};
use tokio::time::{sleep, Duration};
use std::fs::File;
use std::io::prelude::*;
use std::io::{self};

use rndiag_core::ping;
use rndiag_core::nslookup;
use rndiag_core::tcp_ping;
use rndiag_metrics::metrics::MetricsLatency;
use rndiag_metrics::ping_metrics;
use rndiag_metrics::resolver_metrics;
use rndiag_metrics::tping_metrics;

pub async fn launch_srv(parsing_time: u64, addr: &str, addr_srv: &str, port: u16, port_srv: u16, filename: &str, output: &str, nb_ping: u16, flag: u8) {
    let addr_string = addr.to_string();
    let output_clone = output.to_string();
    let addr_srv_string = addr_srv.to_string();
    let filename_string = filename.to_string();

    // Spawn the metrics collection task in the background
    // Main task to launch tools + metrics object, write in metrics file and init warp web-server
    tokio::spawn(async move {
    
        loop {

            // Initialize objects ONCE outside the loop as mutable
            let mut ping = ping::PingTool::new(&addr_string, &output_clone, nb_ping);
            let mut tping = tcp_ping::TCPPingTool::new(&addr_string, &output_clone, nb_ping, port, flag);
            let mut nping = nslookup::NSlookup::new(&addr_string, &output_clone, nb_ping);

            // Run the tool
            ping.run().await.unwrap_or_else(|e|{
                eprintln!("Error during rndiag launching: {}", e);
            });

            tping.run().await.unwrap_or_else(|e|{
                eprintln!("Error during rndiag launching: {}", e);
            });

            nping.run().await.unwrap_or_else(|e|{
                eprintln!("Error during rndiag launching: {}", e);
            });

            // Create equivalents metrics objects with mutable
            let mut pmetrics = ping_metrics::PingMetrics::new(ping.latency_moy_sampled()[0], addr_string.clone());
            let mut tmetrics = tping_metrics::TPingMetrics::new(tping.latency_moy_sampled()[0], addr_string.clone());
            let mut rmetrics = resolver_metrics::NSLookupMetrics::new(nping.latency_moy_sampled()[0], addr_string.clone());

            //Run metrics tools => take and process needed data
            pmetrics.run().unwrap_or_else(|e|{
                eprintln!("Error during rndiag metrics building: {}", e);
            });

            tmetrics.run().unwrap_or_else(|e|{
                eprintln!("Error during rndiag metrics building: {}", e);
            });

            rmetrics.run().unwrap_or_else(|e|{
                eprintln!("Error during rndiag metrics building: {}", e);
            });

            //Create metrics file
            let metrics_file = File::create(&output_clone).unwrap_or_else(|e| {
                eprintln!("Error during the metrics file creation: {}", e);
                std::process::exit(1);
            });

            //Create a writer to write later in the metrics file
            let mut writer = io::BufWriter::new(metrics_file);

            /*Use output_exporter atribute object that contain metrics + desc formated as prometheus text format to write
              in metrics file
             */
            for elem in pmetrics.output_exporter(){
                writer.write_all(elem.as_bytes()).unwrap_or_else(|e| {
                eprintln!("Error during metrics writing in file !: {}", e);
                std::process::exit(1);
                });
            }

            for elem in tmetrics.output_exporter(){
                writer.write_all(elem.as_bytes()).unwrap_or_else(|e| {
                eprintln!("Error during metrics writing in file !: {}", e);
                std::process::exit(1);
                });
            }

            for elem in rmetrics.output_exporter(){
                writer.write_all(elem.as_bytes()).unwrap_or_else(|e| {
                eprintln!("Error during metrics writing in file !: {}", e);
                std::process::exit(1);
                });
            }

            writer.flush().unwrap_or_else(|e| {
                eprintln!("Error during the metrics file closing: {}", e);
            });
            
            //Each parsing_time value in sec, launch again tools and metrics object to refresh metrics
            sleep(Duration::from_secs(parsing_time)).await;
        }
    });

    // Create Arc for sharing later data betsween tasks
    let fname_shared = Arc::new(filename_string.clone());
    
    //Use mutex to share data between tasks
    let route = {
        let fname_shared_clone = fname_shared.clone();
        warp::path(filename_string.clone())
            .and(warp::get())
            .and_then(move || handle_serve_ips(fname_shared_clone.clone()))
    };

    //Split given addr to get each IP bit individually (needed for warp web-server crate)
    let ip_srv_vec_str: Vec<&str> = addr_srv_string.split('.').collect();
    let mut ip_srv_vec_u8: Vec<u8> = Vec::new();
    for elem in ip_srv_vec_str {
        ip_srv_vec_u8.push(elem.parse().unwrap());
    }
    
    println!("\n\nWeb server is running: http://{}:{}/{}\n\n", addr_srv, port_srv, filename);
    
    //Launch the web-server with warp (IP + port)
    warp::serve(route)
        .run(([ip_srv_vec_u8[0], ip_srv_vec_u8[1], ip_srv_vec_u8[2], ip_srv_vec_u8[3]], port_srv))
        .await;
}

//Manage response of webserver when the program receive a request
async fn handle_serve_ips(filename: Arc<String>) -> Result<impl warp::Reply, Infallible> {
    match std::fs::read_to_string(&*filename) {
        Ok(content) => {
            let response = warp::reply::with_header(content, "Content-Type", "text/plain");
            Ok(response.into_response())
        }
        Err(_) => {
            let response = warp::reply::with_status("File not found", StatusCode::NOT_FOUND);
            Ok(response.into_response())
        }
    }
}
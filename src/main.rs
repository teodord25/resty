use std::{env, fs, process::exit};
use colored::Colorize;
use std::time::Duration;

mod types;
mod client;
mod helpers;
use types::*;
use client::Client;
use helpers::*;

#[tokio::main]
async fn main() {
    // load file names
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("{}: missing file name",  "error".red().bold());
        exit(1);
    }

    for test_config in args.iter().skip(1){
        println!("Running {}", test_config);
        println!("----------------------------------------");
        // check if the file/path exists
        let file_contents = fs::read_to_string(test_config);
        if file_contents.is_err(){
            println!("{}: opening file: {}",  "error".red().bold(), file_contents.err().unwrap());
            exit(1);
        }
        let file_contents = file_contents.unwrap();

        // parsing config file into json
        let master_struct: Result<MasterStruct, serde_json::Error> = serde_json::from_str(&file_contents);
        if master_struct.is_err(){
            println!("{}: parsing config file: {}",  "error".red().bold(), master_struct.err().unwrap());
            exit(1);
        }
        let master_struct = master_struct.unwrap();
        let master_client = Client::new(&master_struct.config);
        if master_client.is_err(){
            println!("{} setting up client: {}", "error".red().bold(), master_client.err().unwrap());
            std::process::exit(1);
        }

        let mut success = 0;
        let mut failed = 0;
        let timeout = if master_struct.config.timeout.is_some(){
            master_struct.config.timeout.unwrap()
        }else{
            0
        };

        let master_client = master_client.unwrap();
        for (i, t) in master_struct.tests.iter().enumerate(){
            let result = master_client.exec_test(t).await;
            if result.is_err(){
                println!("{}: executing a test {}", "error".red().bold(), i+1);
                failed += 1;
            }else{
                let result = result.unwrap();
                let mut failed_check: bool = false;

                // check status code
                if t.response_code != result.status().as_u16(){
                    println!("{} ({}) - response code not matching {} != {}",
                    "fail".red().bold(), i+1, t.response_code, result.status().as_u16());
                    failed_check = true;
                }

                // check headers if required
                if let Some(test_headers) = &t.response_headers{
                    let mut first: bool = false;
                    let res_headers = result.headers();
                    for (header_i, h) in test_headers.iter().enumerate(){
                        if !header_match(h,  res_headers){
                            if !first{
                                println!("{} ({}) - headers not matching:", "fail".red().bold(), i+1);
                                failed_check = true;
                                first = true;
                            }
                            println!("\t({}) {} not matching ", header_i+1, h.header);
                            if let Some(value) = res_headers.get(&h.header){
                                println!("\t\tTest header value: {}", h.value);
                                println!("\t\tResult header value: {}", value.to_str().unwrap());
                            }else{
                                println!("\t\t  missing header");
                            }
                        }
                    }
                }

                // check body if required
                if let Some(body) = &t.response_body{
                    let res_body = result.bytes().await;
                    if res_body.is_err(){
                        println!("{} ({}) - error getting response body", "fail".red().bold(), i+1);
                        failed_check = true;
                    }else{
                        let res_body = res_body.unwrap();
                        let res_body_str = res_body.iter().map(|b| *b as char).collect::<String>();
                        if body_match(body, &res_body_str, i){
                            failed_check = true;
                        }
                    }
                }

                if failed_check{
                    failed += 1;
                }else{
                    success += 1;
                    println!("{} ({}) - /{}", "success".green().bold(), i+1, t.request_end_point);
                }
            }

            if timeout > 0{
                std::thread::sleep(Duration::from_millis(timeout as u64));
            }
        }

        println!("\nResults ({}) -> {}: {} {}: {}",
            test_config,
            "success".green().bold(),
            success,
            "failed".red().bold(),
            failed);
        println!("----------------------------------------\n");
    }
}

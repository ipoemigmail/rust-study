use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::sync::Arc;
use std::env;

use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::process::Command;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Hosts {
    hosts: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    //let path = "/Users/ben.jeong/Develop/Works/story/story-deploy/projects/story-app-http/hosts/production.yml";
    //let path = "/tmp/common.yml";
    if args.len() < 2 {
      eprintln!("Usage: {} {{host-yaml-file-path}} cmds...", args[0]);
      std::process::exit(-1);
    }
    let path = args[1].as_str();
    let raw_cmds: Vec<_> = args.iter().skip(2).map(|s| s.as_str()).collect();
    let hosts = get_hosts(path)?;
    let cmds: Arc<Vec<_>> = Arc::new(
       raw_cmds 
            .into_iter()
            .map(|x| x.to_owned())
            .collect(),
    );

    let fibers = hosts
        .iter()
        .map(move |host| tokio::spawn(run_ssh_command(host.clone(), cmds.clone())))
        .collect::<Vec<_>>();

    let results: Vec<_> = stream::iter(fibers)
        .then(|f| async move { f.await.unwrap().unwrap_or_else(|x| x) })
        .collect()
        .await;

    results.iter().for_each(|line| {
      println!("============================== {} ==============================", line.0);
      println!("{}", line.1)
    });

    Ok(())
}

fn get_hosts(path: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let file = File::open(path)?;
    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();
    buf_reader.read_to_string(&mut contents)?;
    let hosts: Hosts = serde_yaml::from_str(&contents)?;
    let result: Vec<_> = hosts.hosts.into_iter().map(|x| x).collect();
    Ok(result)
}

async fn run_ssh_command(host: String, args: Arc<Vec<String>>) -> Result<(String, String), (String, String)> {
    let formatted_args = format!(
        "ssh deploy@{} -o StrictHostKeyChecking=no {}",
        host,
        args.join(" ")
    );
    let v = vec!["-c", formatted_args.as_str()];
    let output = Command::new("sh")
        .args(v)
        .output()
        .await
        .or_else(|e| Err((host.as_str().to_string(), e.to_string())))?;
    let out = output.stdout.into_iter().map(|c| c as char).collect::<String>();
    let err = output.stderr.into_iter().map(|c| c as char).collect::<String>();
    if err.len() > 0 {
        Err((host, err))
    } else {
        Ok((host, out.trim().to_string()))
    }
}

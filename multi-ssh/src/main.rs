use std::{error::Error, string};
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::env;

use futures::{FutureExt, stream::{self, StreamExt}};
use openssh::{KnownHosts, Session};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
enum MyError {
    OpenSshError(openssh::Error),
    JoinError(tokio::task::JoinError),
    FromUtf8Error(string::FromUtf8Error),
}

impl From<openssh::Error> for MyError {
    fn from(e: openssh::Error) -> Self {
        MyError::OpenSshError(e)
    }
}

impl From<tokio::task::JoinError> for MyError {
    fn from(e: tokio::task::JoinError) -> Self {
        MyError::JoinError(e)
    }
}

impl From<string::FromUtf8Error> for MyError {
    fn from(e: string::FromUtf8Error) -> Self {
        MyError::FromUtf8Error(e)
    }
}

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
      eprintln!("Usage: {} {{host-yaml-file-path}} cmd", args[0]);
      std::process::exit(-1);
    }
    let path = args[1].as_str();
    let cmd = args[2].as_str();
    let hosts = get_hosts(path)?;

    let fibers = hosts
        .iter()
        .map(move |host| run_cmd(host.clone(), cmd.to_string()).map(move |x| x.map(|y| (host.clone(), y))))
        .collect::<Vec<_>>();

    let results: Vec<_> = stream::iter(fibers)
        .then(|f| async move { f.await.unwrap() })
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

async fn run_cmd(cmd: String, host: String) -> Result<String, MyError> {
    let handler = tokio::spawn(async move {
        let session = Session::connect(host.clone(), KnownHosts::Accept).await?;
        let child = session.shell(cmd).output().await?;
        let r = String::from_utf8(child.stdout)?;
        Ok(r)
    });
    handler.await?
}
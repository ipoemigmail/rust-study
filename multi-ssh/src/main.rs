use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::sync::Arc;

use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::process::Command;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Hosts {
    hosts: Vec<String>
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    //let path = "/Users/ben.jeong/Develop/Works/story/story-deploy/projects/story-app-http/hosts/production.yml";
    let path = "/tmp/common.yml";
    let hosts = get_hosts(path);
    let cmds: Arc<Vec<_>> = Arc::new(vec!["nodetool gossipinfo | grep generation"]
        .into_iter()
        .map(|x| x.to_owned())
        .collect());
    let results = hosts.into_iter().map(move |host|
        tokio::spawn(run_ssh_command(host, cmds.clone()))
    ).collect::<Vec<_>>();
    let r: Vec<_> = stream::iter(results)
        .then(|f| async move { f.await.unwrap().unwrap() }).collect().await;
    let mat: Vec<_> = r.iter()
        .map(|s| {
            s.split("\n")
                .map(|x| x.split(":").last().unwrap())
                .collect::<Vec<_>>()
        })
        .collect();
    let tmat = transpose(mat);
    let lines: Vec<String> = tmat.iter().map(|row| row.join("\t") ).collect();
    lines.iter().for_each(|line| println!("{}", line));
    Ok(())
}

fn transpose(mat: Vec<Vec<&str>>) -> Vec<Vec<&str>> {
    let mut result = Vec::new();
    for (i, _) in mat.iter().enumerate() {
        let mut row = Vec::new();
        for (j, _) in mat[i].iter().enumerate() {
            row.push(mat[j][i]);
        }
        result.push(row);
    }
    result
}

fn get_hosts(path: &str) -> Vec<String> {
    let file = File::open(path).unwrap();
    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();
    buf_reader.read_to_string(&mut contents).unwrap();
    let hosts: Hosts = serde_yaml::from_str(&contents).unwrap();
    hosts.hosts.into_iter().map(|x| x).collect()
}

async fn run_ssh_command(host: String, args: Arc<Vec<String>>) -> Result<String, String> {
    let formatted_args = format!("ssh deploy@{} -o StrictHostKeyChecking=no {}", host, args.join(" "));
    let v = vec!["-c", formatted_args.as_str()];
    let output = Command::new("sh").args(v).output().await.or_else(|e| Err(e.to_string()))?;
    let out = output.stdout.iter().map(|c| *c as char).collect::<String>();
    let err = output.stderr.iter().map(|c| *c as char).collect::<String>();
    if err.len() > 0 {
        Err(err)
    } else {
        Ok(out.trim().to_string())
    }
}

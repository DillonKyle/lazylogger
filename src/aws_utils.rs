use aws_sdk_ecs::{
    Client, Error,
    operation::{
        describe_clusters::DescribeClustersOutput, describe_services::DescribeServicesOutput,
    },
    types::Service,
};
use color_eyre::Result;
use itertools::Itertools;
use std::{
    error,
    fs::File,
    io::{self, BufRead},
};
pub async fn get_profiles() -> Result<Vec<String>, Box<dyn error::Error>> {
    let mut profiles = Vec::new();
    let cred_file = File::open(dirs::home_dir().unwrap().join(".aws").join("credentials")).unwrap();
    let read_creds = io::BufReader::new(cred_file);
    for line in read_creds.lines() {
        let line = line.unwrap();
        if line.starts_with('[') && line.ends_with(']') {
            let profile = line.trim_matches(&['[', ']'][..]);
            profiles.push(profile.to_string());
        }
    }
    profiles.sort();
    Ok(profiles)
}

pub async fn get_clusters(client: &Client) -> Result<DescribeClustersOutput, Error> {
    let resp = client.list_clusters().send().await?;
    let mut cluster_arns = resp.cluster_arns().to_vec();
    cluster_arns.sort();
    let cluster = client
        .describe_clusters()
        .set_clusters(Some(cluster_arns))
        .send()
        .await?;
    Ok(cluster)
}

pub async fn get_services(
    client: &Client,
    cluster_name: &str,
) -> Result<DescribeServicesOutput, Error> {
    let mut next_token = None;
    let mut service_arns: Vec<String> = Vec::new();

    loop {
        let resp = client
            .list_services()
            .cluster(cluster_name)
            .set_next_token(next_token.clone())
            .send()
            .await?;

        service_arns.extend(resp.service_arns().to_vec());

        if let Some(token) = resp.next_token() {
            next_token = Some(token.to_string());
        } else {
            break;
        }
    }

    service_arns.sort();
    let mut all_services: Vec<_> = Vec::new();

    for chunk in &service_arns.into_iter().chunks(10) {
        let resp = client
            .describe_services()
            .cluster(cluster_name)
            .set_services(Some(chunk.collect()))
            .send()
            .await?;
        if let Some(s) = resp.services {
            all_services.extend(s);
        }
    }

    let output = DescribeServicesOutput::builder()
        .set_services(Some(all_services))
        .build();

    Ok(output)
}

/*
async fn get_tasks(
    client: &Client,
    cluster_name: &str,
    service_name: &str,
) -> Result<DescribeTasksOutput, Error> {
    let resp = client
        .list_tasks()
        .cluster(cluster_name)
        .service_name(service_name)
        .send()
        .await?;
    let task_arns = resp.task_arns();
    let tasks = client
        .describe_tasks()
        .cluster(cluster_name)
        .set_tasks(Some(task_arns.into()))
        .send()
        .await?;
    Ok(tasks)
}
*/

pub async fn get_events(service: &Service) -> Result<Vec<String>, Error> {
    let logs = service.events();
    let mut formatted_logs = Vec::new();
    for entry in logs {
        formatted_logs.push(format!(
            "[{}] {}",
            entry.created_at().unwrap(),
            entry.message().unwrap(),
        ));
    }
    Ok(formatted_logs)
}

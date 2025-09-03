use aws_sdk_cloudwatchlogs as cloudwatch;
use aws_sdk_ecs::{
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

pub async fn get_clusters(
    client: &aws_sdk_ecs::Client,
) -> Result<DescribeClustersOutput, aws_sdk_ecs::Error> {
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
    client: &aws_sdk_ecs::Client,
    cluster_name: &str,
) -> Result<DescribeServicesOutput, aws_sdk_ecs::Error> {
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

pub async fn get_log_group_name(
    ecs_client: &aws_sdk_ecs::Client,
    service: &Service,
) -> Result<String, aws_sdk_ecs::Error> {
    let task_def_arn = service.task_definition().unwrap();
    let task_def = ecs_client
        .describe_task_definition()
        .task_definition(task_def_arn)
        .send()
        .await?;
    let container_defs = task_def.task_definition().unwrap().container_definitions();
    let log_config = container_defs[0].log_configuration().unwrap();
    let log_group = log_config.options().unwrap().get("awslogs-group").unwrap();
    Ok(log_group.clone())
}

pub async fn get_logs(
    cw_client: &cloudwatch::Client,
    log_group: &String,
) -> Result<Vec<String>, cloudwatch::Error> {
    let log_events = cw_client
        .filter_log_events()
        .log_group_name(log_group)
        .limit(500)
        .send()
        .await;
    let mut logs = Vec::new();
    if let Some(events) = log_events.unwrap().events {
        for event in events {
            logs.push(format!(
                "[{}] {}",
                event.timestamp.unwrap(),
                event.message.unwrap()
            ));
        }
    }

    Ok(logs)
}

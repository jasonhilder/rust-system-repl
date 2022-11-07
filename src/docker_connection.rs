use std::collections::HashMap;

use bollard::{
    Docker,
    image::CreateImageOptions,
    container::{
        ListContainersOptions,
        StartContainerOptions,
        CreateContainerOptions,
        Config
    },
    exec::{
        StartExecResults,
        CreateExecOptions
    }
};

use druid::{Target, Selector};
use futures_util::{TryStreamExt, StreamExt};

const IMAGE: &str = "alpine:latest";
pub const CONTAINER_NAME: &str = "rusty-repl";
pub const UPDATE_MSG: Selector<String> = Selector::new("update_message");

pub async fn setup_container(event_sink: druid::ExtEventSink) {
    println!("starting setup");

    let docker = Docker::connect_with_local_defaults();

    if let Ok(docker) = docker {

        let mut filters = HashMap::new();
        filters.insert("name", vec![CONTAINER_NAME]);

        let options = Some(ListContainersOptions{
            all: true,
            filters,
            ..Default::default()
        });

        let container_list = docker.list_containers(options).await.unwrap();

        if container_list.len() > 0 {
            event_sink.submit_command(
                UPDATE_MSG,
                "starting container...".to_string(),
                Target::Auto
            ).expect("command failed to submit");

            // container exists just start it.
            let container_state = docker.start_container(
                CONTAINER_NAME,
                None::<StartContainerOptions<String>>
            ).await;

            if container_state.is_ok() {
                event_sink.submit_command(
                    UPDATE_MSG,
                    "container running".to_string(),
                    Target::Auto
                ).expect("command failed to submit");
                return
            } else {
                eprintln!("failed to connect to docker");
            }

        } else {
            //container does not exist, create it
            //and add node to it
            create_container(&docker, event_sink).await
        }

    } else {
        eprintln!("failed to connect to docker");
    }
}

async fn create_container(docker: &Docker, event_sink: druid::ExtEventSink) {
    update_ui_detail_msg(&event_sink, "downloading");

    docker
        .create_image(
            Some(CreateImageOptions {
                from_image: IMAGE,
                ..Default::default()
            }),
            None,
            None,
        )
        .try_collect::<Vec<_>>()
        .await.unwrap();

    let container_ops = CreateContainerOptions {
        name:CONTAINER_NAME,
    };

    let alpine_config = Config {
        image: Some(IMAGE),
        tty: Some(true),
        attach_stdin: Some(true),
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        open_stdin: Some(true),
        ..Default::default()
    };

    let id = docker.create_container::<&str, &str>(Some(container_ops), alpine_config).await.unwrap().id;
    let has_started = docker.start_container::<String>(&id, None).await;

    if has_started.is_ok() {
        println!("completed setup");
        update_ui_detail_msg(&event_sink, "completed setup");

        // non interactive exec setup node
        let setup_node = docker
            .create_exec(
                CONTAINER_NAME,
                CreateExecOptions {
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    cmd: Some(vec!["apk", "add", "--update", "nodejs", "npm"]),
                    ..Default::default()
                },
            )
            .await.unwrap()
            .id;

        if let StartExecResults::Attached { mut output, .. } = docker.start_exec(&setup_node, None).await.unwrap() {
            while let Some(Ok(msg)) = output.next().await {
                print!("{}", msg);
                update_ui_detail_msg(&event_sink, &msg.to_string());
            }
        } else {
            unreachable!();
        }

        // create temp folder location
        let setup_node_path = docker
            .create_exec(
                CONTAINER_NAME,
                CreateExecOptions {
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    cmd: Some(vec![
                        "mkdir", "-p", "rusty-repl/node/",
                        "&&", "touch","rusty-repl/node/main.js"
                    ]),
                    ..Default::default()
                },
            )
            .await.unwrap()
            .id;

        if let StartExecResults::Attached { mut output, .. } = docker.start_exec(&setup_node_path, None).await.unwrap() {
            while let Some(Ok(msg)) = output.next().await {
                print!("{}", msg);
                update_ui_detail_msg(&event_sink, &msg.to_string());
            }
        } else {
            unreachable!();
        }

    } else {
        eprintln!("failed to start docker container")
    }
}

fn update_ui_detail_msg(event_sink: &druid::ExtEventSink, message: &str) {
    event_sink.submit_command(
        UPDATE_MSG,
        message.to_string(),
        Target::Auto
    ).expect("command failed to submit");
}

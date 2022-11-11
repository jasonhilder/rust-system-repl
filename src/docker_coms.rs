use std::{collections::HashMap, fs};

use bollard::{
    Docker,
    image::BuildImageOptions,
    container::{
        ListContainersOptions,
        StartContainerOptions,
        CreateContainerOptions,
        Config
    },
    exec::{
        StartExecResults,
        CreateExecOptions
    }, service::{Mount, HostConfig, MountTypeEnum}
};

use druid::{Target, Selector, Command};
use futures_util::{TryStreamExt, StreamExt};

use std::fs::File;
use std::io::Read;

const IMAGE: &str = "rusty-repl-image";
pub const CONTAINER_NAME: &str = "rusty-repl";
pub const UPDATE_MSG: Selector<String> = Selector::new("update_message");
pub const DOCKER_EXEC: Selector<String> = Selector::new("exec_docker");

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
            println!("creating container");
            //container does not exist, create it
            //and add node to it
            create_container(&docker, event_sink).await
        }

    } else {
        eprintln!("failed to connect to docker");
    }
}

async fn create_container(docker: &Docker, event_sink: druid::ExtEventSink) {
    update_ui_detail_msg(&event_sink, "building image...");

    let options = BuildImageOptions{
        dockerfile: "Dockerfile".to_string(),
        t: IMAGE.to_string(),
        pull: true,
        rm: false,
        ..Default::default()
    };

    let mut file = File::open("./docker_files/node.tar.gz").unwrap();
    let mut contents = Vec::new();
    file.read_to_end(&mut contents).unwrap();

    docker.build_image(options, None, Some(contents.into())).try_collect::<Vec<_>>().await.unwrap();

    let container_ops = CreateContainerOptions {
        name:CONTAINER_NAME,
    };

    let mut mount_points:HashMap<String, HashMap<(), ()>>  = HashMap::new();
    let mut mount_paths = HashMap::new();
    mount_paths.insert((), ());
    mount_points.insert(String::from("rusty-tester:/rusty-rep"), mount_paths);

    let host_cfg = HostConfig {
        mounts: Some(
            vec![
                Mount {
                    target: Some("/rusty-rep".to_string()),
                    source: Some("/home/jason/rusty-tester".to_string()),
                    typ: Some(MountTypeEnum::BIND),
                    consistency: Some(String::from("default")),
                    ..Default::default()
                }
            ]
        ),
        ..Default::default()
    };

    let alpine_config = Config {
        image: Some(IMAGE),
        host_config: Some(host_cfg),
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

    } else {
        eprintln!("failed to start docker container")
    }
}

pub async fn docker_exec_program(code: String) -> Option<String> {
    let docker = Docker::connect_with_local_defaults().unwrap();

    // first write to file
    fs::write("/home/jason/rusty-tester/main.js", code).expect("Unable to write file");

    // execute node
    let x = docker.create_exec(
        CONTAINER_NAME,
        CreateExecOptions {
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            cmd: Some(vec!["node", "rusty-rep/main.js"]),
            ..Default::default()
        },
    )
    .await.unwrap()
    .id;

    if let StartExecResults::Attached { mut output, .. } = docker.start_exec(&x, None).await.unwrap() {
        while let Some(Ok(msg)) = output.next().await {
            print!("execute {}", msg);
            return Some(msg.to_string());
        }
        None
    } else {
        unreachable!();
    }
}

pub fn exec_cmd(code: &String) -> Command {
    Command::new(
        DOCKER_EXEC,
        code.clone(),
        Target::Auto
    )
}

fn update_ui_detail_msg(event_sink: &druid::ExtEventSink, message: &str) {
    event_sink.submit_command(
        UPDATE_MSG,
        message.to_string(),
        Target::Auto
    ).expect("command failed to submit");
}

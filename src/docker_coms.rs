use std::{
    collections::HashMap,
    fs
};
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
use druid::{
    Target,
    Command
};
use futures_util::{
    TryStreamExt,
    StreamExt
};
use std::fs::File;
use std::io::Read;

use crate::RsrEvent;

const IMAGE: &str = "rusty-repl-image";
const LOCAL_PATH: &str = "/home/jason/rusty-tester/";
pub const CONTAINER_NAME: &str = "rusty-repl";

pub async fn setup_container(event_sink: &druid::ExtEventSink) {
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
                crate::UPDATE_MSG,
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
                    crate::UPDATE_MSG,
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

async fn create_container(docker: &Docker, event_sink: &druid::ExtEventSink) {
    update_ui_detail_msg(&event_sink, "building image...");

    let options = BuildImageOptions{
        dockerfile: "Dockerfile".to_string(),
        t: IMAGE.to_string(),
        pull: true,
        rm: true,
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
                    source: Some(LOCAL_PATH.to_string()),
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

pub fn docker_handle_event(e: RsrEvent, event_sink: &druid::ExtEventSink) {

    let es = event_sink.clone();

    match e {
        RsrEvent::Exec(code) => {
            tokio::spawn(async move {
                let std_out = docker_exec_program(code).await;

                if let Some(out) = std_out {
                    es.submit_command(
                        crate::UPDATE_OUTPUT,
                        out.to_string(),
                        Target::Auto
                    ).expect("command failed to submit");
                }
            });

            ()
        },
        RsrEvent::ImportLibs(imports) => {
            tokio::spawn(async move {
                println!("imports: {}", imports);
                let std_out = docker_import_libs(imports).await;
                println!("std out: {:?}", std_out);
            });

            ()
        }
    }
}

pub async fn docker_import_libs(imports: String) -> Option<String> {
    let docker = Docker::connect_with_local_defaults().unwrap();

    let mut import_command = vec!["cd", "/rusty-rep"];

    // for imprt in imports.split("\n").into_iter() {
    //     import_command.push(imprt)
    // }
    println!("cmd: {:#?}", import_command);

    let x = docker.create_exec(
        CONTAINER_NAME,
        CreateExecOptions {
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            cmd: Some(import_command),
            ..Default::default()
        },
    )
    .await.unwrap()
    .id;

    if let StartExecResults::Attached { mut output, .. } = docker.start_exec(&x, None).await.unwrap() {
        let mut txt = String::new();

        while let Some(Ok(msg)) = output.next().await {
            txt.push_str(&msg.to_string());
        }

        //println!("output = {}", txt);
        Some(txt)
    } else {
        unreachable!();
    }
}

pub async fn docker_exec_program(code: String) -> Option<String> {
    let docker = Docker::connect_with_local_defaults().unwrap();

    let file_path = format!("{}/main.js", LOCAL_PATH);
    // first write to file
    fs::write(file_path, code.trim()).expect("Unable to write file");

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
        let mut txt = String::new();

        while let Some(Ok(msg)) = output.next().await {
            txt.push_str(&msg.to_string());
        }
        //println!("output = {}", txt);
        Some(txt)
    } else {
        unreachable!();
    }
}

pub fn submit_rsr_event(event: RsrEvent) -> Command {
    Command::new(
        crate::RSR_EVENT,
        event,
        Target::Auto
    )
}

fn update_ui_detail_msg(event_sink: &druid::ExtEventSink, message: &str) {
    event_sink.submit_command(
        crate::UPDATE_MSG,
        message.to_string(),
        Target::Auto
    ).expect("command failed to submit");
}

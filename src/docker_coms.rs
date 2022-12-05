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

use tokio::runtime::Handle;
use tokio::task;
use futures_util::StreamExt;
use std::fs::File;
use std::io::Read;

use crate::RsrEvent;
const IMAGE: &str = "rusty-rep/nodejs";
const LOCAL_PATH: &str = "/home/jason/rusty-tester/";
pub const CONTAINER_NAME: &str = "rusty-repl";

pub async fn setup_container(event_sink: &druid::ExtEventSink, docker_connection: &Docker) {
    println!("starting setup");

    let mut filters = HashMap::new();
    filters.insert("name", vec![CONTAINER_NAME]);

    let options = Some(ListContainersOptions{
        all: true,
        filters,
        ..Default::default()
    });

    let container_list = docker_connection.list_containers(options).await.unwrap();

    if container_list.len() > 0 {
        event_sink.submit_command(
            crate::UPDATE_MSG,
            "starting container...".to_string(),
            Target::Auto
        ).expect("command failed to submit");

        // container exists just start it.
        let container_state = docker_connection.start_container(
            CONTAINER_NAME,
            None::<StartContainerOptions<String>>
        ).await;

        if container_state.is_ok() {
            event_sink.submit_command(
                crate::UPDATE_MSG,
                "container running".to_string(),
                Target::Auto
            ).expect("command failed to submit");

            docker_handle_imports(event_sink);
            return
        } else {
            eprintln!("failed to connect to docker");
        }

    } else {
        println!("creating container");
        //container does not exist, create it
        //and add node to it
        create_container(&docker_connection, event_sink).await;
        docker_handle_imports(event_sink);
    }

}

async fn create_container(docker: &Docker, event_sink: &druid::ExtEventSink) {
    update_ui_detail_msg(&event_sink, "building image...");

    // create local folder
    fs::create_dir_all(LOCAL_PATH).unwrap();

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

    docker.build_image(options, None, Some(contents.into())).for_each(|x| async move { println!("{:?}", x)}).await;

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

pub fn docker_handle_event(e: RsrEvent, event_sink: &druid::ExtEventSink, docker: &Docker) {

    println!("in handler");
    println!("{:?}", e);
    let es = event_sink.clone();

    match e {
        RsrEvent::Exec(code) => {
            task::block_in_place(move || {
                Handle::current().block_on(async move {
                    println!("execing!");
                    es.submit_command(
                        crate::START_PROCESSING,
                        None,
                        Target::Auto
                    ).expect("command failed to submit");

                    let std_out = docker_exec_program(code, docker).await;

                    if let Some(out) = std_out {
                        es.submit_command(
                            crate::UPDATE_OUTPUT,
                            out.to_string(),
                            Target::Auto
                        ).expect("command failed to submit");

                        es.submit_command(
                            crate::END_PROCESSING,
                            None,
                            Target::Auto
                        ).expect("command failed to submit");
                    }
                })
            });

            ()
        },
        RsrEvent::ImportLibs(imports) => {
            println!("running here1");

            task::block_in_place(move || {
                Handle::current().block_on(async move {
                    let std_out = docker_import_libs(imports, docker).await;
                    println!("std out: {:?}", std_out);

                    if let Some(res) = std_out {
                        es.submit_command(
                            crate::UPDATE_OUTPUT,
                            res,
                            Target::Auto
                        ).expect("command failed to submit");
                    }
                });
            });

            ()
        },
        RsrEvent::Start() => {
            task::block_in_place(move || {
                Handle::current().block_on(async move {
                    setup_container(&es, docker).await;
                })
            });
        }

    }
}

fn docker_handle_imports(event_sink: &druid::ExtEventSink) {
    let file_path = format!("{}/package.json", LOCAL_PATH);
    // first write to file
    let fc = fs::read_to_string(file_path).expect("Unable to read file");

    set_import_text(event_sink, fc.as_str());
}

pub async fn docker_import_libs(import_txt: String, docker_connection: &Docker) -> Option<String> {
    println!("importing libs");


    let file_path = format!("{}/package.json", LOCAL_PATH);
    // first write to file
    fs::write(file_path, import_txt).expect("Unable to write file");

    let import_command = vec!["npm", "i"];

    let x = docker_connection.create_exec(
        CONTAINER_NAME,
        CreateExecOptions {
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            cmd: Some(import_command),
            working_dir: Some("/rusty-rep"),
            ..Default::default()
        },
    )
    .await.unwrap()
    .id;

    if let StartExecResults::Attached { mut output, .. } = docker_connection.start_exec(&x, None).await.unwrap() {
        let mut txt = String::new();

        while let Some(Ok(msg)) = output.next().await {
            txt.push_str(&msg.to_string());
        }

        Some(txt)
    } else {
        unreachable!();
    }
}

pub async fn docker_exec_program(code: String, docker_connection: &Docker) -> Option<String> {

    let file_path = format!("{}/main.js", LOCAL_PATH);
    // first write to file
    fs::write(file_path, code.trim()).expect("Unable to write file");

    // execute node
    let x = docker_connection.create_exec(
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

    if let StartExecResults::Attached { mut output, .. } = docker_connection.start_exec(&x, None).await.unwrap() {
        let mut txt = String::new();

        while let Some(Ok(msg)) = output.next().await {
            txt.push_str(&msg.to_string());
        }

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

fn set_import_text(event_sink: &druid::ExtEventSink, file_content: &str) {
    event_sink.submit_command(
        crate::SETUP_IMPORTS,
        file_content.to_string(),
        Target::Auto
    ).expect("command failed to submit");
}

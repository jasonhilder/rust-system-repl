mod docker_coms;
mod gui;

use bollard::{container::StopContainerOptions, Docker};
use druid::{AppDelegate, AppLauncher, Command, DelegateCtx, Env, Handled, Selector, Target};
use gui::{build_window, AppState};
use std::sync::mpsc::{channel, Sender};

struct Delegate {
    tx: Sender<RsrEvent>,
}

pub const UPDATE_MSG: Selector<String> = Selector::new("update_message");
pub const UPDATE_OUTPUT: Selector<String> = Selector::new("update_output");
pub const END_PROCESSING: Selector<Option<&str>> = Selector::new("end_processing");
pub const START_PROCESSING: Selector<Option<&str>> = Selector::new("start_processing");
pub const RSR_EVENT: Selector<RsrEvent> = Selector::new("exec_docker");

impl AppDelegate<AppState> for Delegate {
    fn command(
        &mut self,
        _ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        data: &mut AppState,
        _env: &Env,
    ) -> Handled {
        if let Some(msg) = cmd.get(UPDATE_MSG) {
            data.loading_msg = msg.clone();
            Handled::Yes
        } else if let Some(out) = cmd.get(UPDATE_OUTPUT) {
            data.output_box = out.clone();
            Handled::Yes
        } else if let Some(_) = cmd.get(START_PROCESSING) {
            println!("starting to process");
            data.processing = true;
            data.edited_timestamp = 1;
            Handled::Yes
        } else if let Some(_) = cmd.get(END_PROCESSING) {
            println!("ending process");
            data.processing = false;
            data.edited_timestamp = 1;
            Handled::Yes
        } else if let Some(event) = cmd.get(RSR_EVENT) {
            let r_event = event.clone();
            let ev = self.tx.send(r_event);

            if ev.is_err() {
                println!("HL {:#?}", ev.err());
            }
            Handled::Yes
        } else {
            Handled::No
        }
    }
}

#[derive(Clone)]
pub enum RsrEvent {
    Exec(String), // <- send this straight to eh docker container
    ImportLibs(String),
}

#[tokio::main]
async fn main() {
    let (tx, rx) = channel::<RsrEvent>();

    // connect to docker in main app
    let docker = Docker::connect_with_local_defaults().unwrap();

    // describe the main window
    let main_window = build_window();
    let launcher = AppLauncher::with_window(main_window);
    let event_sink = launcher.get_external_handle();

    // create the initial app state
    let initial_state = AppState {
        import_box_chars: 0,
        import_box: "".to_string(),
        text_box: "".to_string(),
        output_box: "".to_string(),
        loading_msg: "".to_string(),
        loading: false,
        processing: false,
        edited_timestamp: 0
    };

    // works with tokio spawn rather than thread::spawn
    // as I need an async function for docker api
    // spawn async process to handle events
    tokio::spawn(async move {
        docker_coms::setup_container(&event_sink).await;

        loop {
            if let Ok(event) = rx.try_recv() {
                docker_coms::docker_handle_event(event, &event_sink)
            }
        }
    });

    // start the application
    launcher
        .delegate(Delegate { tx })
        .launch(initial_state)
        .expect("Failed to launch application");

    docker
        .stop_container(
            docker_coms::CONTAINER_NAME,
            Some(StopContainerOptions { t: 5 }),
        )
        .await
        .unwrap();
}

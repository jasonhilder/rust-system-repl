mod gui;
mod docker_coms;

use gui::{build_window, AppState};
use bollard::{
    Docker,
    container::StopContainerOptions
};
use druid::{
    AppDelegate,
    AppLauncher,
    Command,
    Env,
    Target,
    DelegateCtx,
    Handled
};

struct Delegate;

impl AppDelegate<AppState> for Delegate {
    fn command(
        &mut self,
        _ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        data: &mut AppState,
        _env: &Env,
    ) -> Handled {
        if let Some(msg) = cmd.get(docker_coms::UPDATE_MSG) {
            // If the command we received is `FINISH_SLOW_FUNCTION` handle the payload.
            println!("Hand1");
            data.loading_msg = msg.clone();
            Handled::Yes

        } else if let Some(code) = cmd.get(docker_coms::DOCKER_EXEC) {
            //println!("execute this code:\n {}", code);

            let x = code.clone();
            tokio::spawn(async {
                docker_coms::docker_exec_program(x).await;
            });

            Handled::Yes

        } else {
            println!("Hand2");
            Handled::No
        }
    }
}


#[tokio::main]
async fn main() {
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
        loading: false,
        loading_msg: "".to_string()
    };

    println!("before");

    // works with tokio spawn rather than thread::spawn
    // as I need an async function for docker api
    // spawn async process to handle events
    tokio::spawn(async {
        docker_coms::setup_container(event_sink).await
    });

    // start the application
    launcher.delegate(Delegate).launch(initial_state).expect("Failed to launch application");

    println!("app closing now");

    docker.stop_container(
        docker_coms::CONTAINER_NAME,
        Some(StopContainerOptions{t: 5})
    ).await.unwrap();
}

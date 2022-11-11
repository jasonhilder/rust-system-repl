mod gui;
mod docker_coms;

use std::{sync::{mpsc::channel, Arc}, sync::{mpsc::Sender, Mutex}};

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

struct Delegate {
    tx: Sender<String>
}

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
            data.loading_msg = msg.clone();
            Handled::Yes

        } else if let Some(code) = cmd.get(docker_coms::DOCKER_EXEC) {
            //println!("execute this code:\n {}", code);
            let x = code.clone();
            let ev = self.tx.send(x);

            if ev.is_err() {
                println!("HL {:?}", ev.err());
            }
            // let h = tokio::task::spawn_blocking(move || {
            //     // let x = docker_coms::docker_exec_program(x).await.unwrap();
            //     // x
            //     spawn(async {
            //         let x = docker_coms::docker_exec_program(x).await.unwrap();
            //         x
            //     })
            // });


            // let res = spawn(async {
            //     let x = docker_coms::docker_exec_program(x).await.unwrap();
            //     data.output_box = x
            // });

            Handled::Yes
        } else {
            Handled::No
        }
    }
}


enum JEvent {
    Exec(String), // <- send this straight to eh docker container
}

#[tokio::main]
async fn main() {

    let (tx,rx) = channel::<String>();

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

    let receiver = Arc::new(Mutex::new(rx));

    // works with tokio spawn rather than thread::spawn
    // as I need an async function for docker api
    // spawn async process to handle events
    tokio::spawn(async move {
        docker_coms::setup_container(event_sink).await;

        // loop {
        //     // match rx.try_recv().unwrap() {
        //     //     JEvent::Exec(a) => {
        //     //         println!("Inside here y'all: {:?}", a);
        //     //         docker_coms::docker_exec_program(a.clone()).await;
        //     //     },
        //     // }
        //     if let Ok(a) = rx.try_recv() {
        //         let s:String = a.lock().unwrap().to_string();
        //         // docker_coms::docker_exec_program(a.clone()).await;
        //     }
        // }
        let irx = receiver.lock().unwrap();
    });

    // start the application
    launcher.delegate(Delegate {
        tx: tx
    }).launch(initial_state).expect("Failed to launch application");

    println!("app closing now");

    docker.stop_container(
        docker_coms::CONTAINER_NAME,
        Some(StopContainerOptions{t: 5})
    ).await.unwrap();
}

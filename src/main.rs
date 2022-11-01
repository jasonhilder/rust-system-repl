use bollard::Docker;
use bollard::container::{Config, CreateContainerOptions};
use bollard::image::CreateImageOptions;
use druid::widget::{Align, Controller, CrossAxisAlignment, Flex, TextBox, Spinner};
use druid::{
    AppDelegate, AppLauncher, Command, Data, Env, Event, EventCtx, Lens, LocalizedString, Widget, WidgetExt, WindowDesc, Selector, Target, DelegateCtx, Handled,
};
use futures_util::TryStreamExt;
use std::{fs, thread};

const VERTICAL_WIDGET_SPACING: f64 = 20.0;
const WINDOW_TITLE: LocalizedString<AppState> = LocalizedString::new("RJSI");
const IMAGE: &str = "alpine:latest";
const UPDATE_MSG: Selector<String> = Selector::new("update_message");

#[derive(Clone, Data, Lens)]
struct AppState {
    import_box_chars: u64,
    import_box: String,
    text_box: String,
    output_box: String,
    loading: bool,
    loading_msg: String,
}

struct ExecuteNodeCode;

impl<W: Widget<AppState>> Controller<AppState, W> for ExecuteNodeCode {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut AppState,
        env: &Env,
    ) {
        child.event(ctx, event, data, env);
        if let Event::KeyDown(_) = event {
            //TODO set some sort of debounce
            //TODO write the text_box to file
            println!("{}", &data.text_box);

            if data.text_box.len() > 0 {
                fs::write("./index.js", &data.text_box).expect("Unable to write file");
                // let out = Command::new("node")
                //     .arg("./index.js")
                //     .output()
                //     .expect("node issues yer");

                //let stdo = String::from_utf8_lossy(&out.stdout);
                // println!("stdout: {}", stdo);
                // data.output_box = stdo.to_string()
            }

            // println!("{}", out.status);
            // println!("stderr: {}", String::from_utf8_lossy(&out.stderr));
        }
    }
}
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
        if let Some(msg) = cmd.get(UPDATE_MSG) {
            // If the command we received is `FINISH_SLOW_FUNCTION` handle the payload.
            println!("Hand1");
            data.loading_msg = msg.clone();
            Handled::Yes
        } else {
            println!("Hand2");
            Handled::No
        }
    }
}

async fn setup_container(event_sink: druid::ExtEventSink) {
    event_sink.submit_command(UPDATE_MSG, "downloading!".to_string(), Target::Auto).expect("command failed to submit");
    println!("starting setup");
    let docker = Docker::connect_with_local_defaults().unwrap();

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
        name:"rusty-repl",
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
        event_sink.submit_command(UPDATE_MSG, "completed setup!".to_string(), Target::Auto).expect("command failed to submit");
    } else {
        panic!("WTF...")
    }
}

fn build_app() -> impl Widget<AppState> {
    let imports_box = TextBox::multiline()
        .with_placeholder("Npm Imports")
        .expand_width()
        .expand_height()
        .lens(AppState::import_box);

    let text_box = TextBox::multiline()
        .with_placeholder("Code here")
        .expand_width()
        .expand_height()
        .lens(AppState::text_box)
        .controller(ExecuteNodeCode);

    let loading = Spinner::new()
        .expand_width()
        .expand_height();

    let detail_box = TextBox::new()
        .with_placeholder("Output")
        .expand_width()
        .expand_height()
        .lens(AppState::loading_msg);

    let output_box = TextBox::new()
        .with_placeholder("Output")
        .expand_width()
        .expand_height()
        .lens(AppState::output_box);

    let left_column = Flex::column()
        .with_flex_child(imports_box, 20.0)
        .with_spacer(VERTICAL_WIDGET_SPACING)
        .with_flex_child(text_box, 80.0)
        .padding(5.0)
        .expand_height()
        .expand_width();

    let right_column_top = Flex::row()
        .with_flex_child(loading, 10.0)
        .with_flex_child(detail_box, 90.0)
        .expand_width();

    let right_column = Flex::column()
        .with_flex_child(right_column_top, 20.0)
        .with_spacer(VERTICAL_WIDGET_SPACING)
        .with_flex_child(output_box, 80.0)
        .padding(5.0)
        .expand_height()
        .expand_width();

    let container = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .must_fill_main_axis(true)
        .with_flex_child(left_column, 50.0)
        .with_default_spacer()
        .with_flex_child(right_column, 50.0)
        .expand_width();

    // center the two widgets in the available space
    Align::centered(container)
}

#[tokio::main]
async fn main() {
    // create the initial app state
    let initial_state = AppState {
        import_box_chars: 0,
        import_box: "".to_string(),
        text_box: "".to_string(),
        output_box: "".to_string(),
        loading: false,
        loading_msg: "".to_string()
    };

    // setup docker

    // describe the main window
    let main_window = WindowDesc::new(build_app)
        .title(WINDOW_TITLE)
        .window_size((400.0, 400.0));

    let launcher = AppLauncher::with_window(main_window);

    let event_sink = launcher.get_external_handle();

    println!("before");

    // works with tokio spawn vs thread::spawn
    // as I need an async function for docker api
    tokio::spawn(async {
        setup_container(event_sink).await
    });

    println!("after");

    // start the application
    launcher.delegate(Delegate).launch(initial_state).expect("Failed to launch application");
}

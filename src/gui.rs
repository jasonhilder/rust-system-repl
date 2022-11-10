use crate::docker_coms;
use bollard::{Docker, exec::{CreateExecOptions, StartExecResults}};
use druid::{
    LocalizedString,
    Widget,
    WidgetExt,
    WindowDesc,
    Data,
    Lens,
    EventCtx,
    Env,
    Event,
    widget:: {
        Align,
        CrossAxisAlignment,
        Flex,
        TextBox,
        Spinner, Controller
    }
};
use futures_util::StreamExt;

const WINDOW_TITLE: LocalizedString<AppState> = LocalizedString::new("RJSI");
const VERTICAL_WIDGET_SPACING: f64 = 20.0;

#[derive(Clone, Data, Lens)]
pub struct AppState {
    pub import_box_chars: u64,
    pub import_box: String,
    pub text_box: String,
    pub output_box: String,
    pub loading: bool,
    pub loading_msg: String,
}

struct ExecuteNodeCode {
    docker_con: Docker
}

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
            ctx.submit_command(docker_coms::exec_cmd(&data.text_box))

            /*
                1. Investigate if you can create an Arc for this and then clone the pointer (you'd need to lock; check tokio docs)
                    then you'd be passing down a ptr clone instead of a full docker struct.
            */

            // let c = self.docker_con.clone();
            // if data.text_box.len() > 0 {
            //     tokio::spawn(async move {
            // }
            // println!("{}", out.status);
            // println!("stderr: {}", String::from_utf8_lossy(&out.stderr));
        }
    }
}

fn build_app() -> impl Widget<AppState> {
    // connect to docker in main app
    let docker = Docker::connect_with_local_defaults().unwrap();

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
        .controller(ExecuteNodeCode{
            docker_con: docker
        });

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

    let right_column_bot = Flex::row()
        .with_flex_child(loading, 5.0)
        .with_flex_child(detail_box, 95.0)
        .expand_width();

    let right_column = Flex::column()
        .with_flex_child(output_box, 95.0)
        .with_spacer(VERTICAL_WIDGET_SPACING)
        .with_flex_child(right_column_bot, 5.0)
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

pub fn build_window() -> WindowDesc<AppState> {
    WindowDesc::new(build_app)
            .title(WINDOW_TITLE)
            .window_size((400.0, 400.0))
}

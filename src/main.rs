use std::fs;
use std::process::Command;
use druid::widget::{Align, Flex, TextBox, CrossAxisAlignment, Controller};
use druid::{
    AppLauncher,
    Data,
    Lens,
    LocalizedString,
    Widget,
    WindowDesc,
    WidgetExt,
    EventCtx,
    Event,
    Env
};

const VERTICAL_WIDGET_SPACING: f64 = 20.0;
const WINDOW_TITLE: LocalizedString<AppState> = LocalizedString::new("RJSI");

#[derive(Clone, Data, Lens)]
struct AppState {
    import_box_chars: u64,
    import_box: String,
    text_box: String,
    output_box: String
}

struct ExecuteNodeCode;

impl<W: Widget<AppState>> Controller<AppState, W> for ExecuteNodeCode {
    fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut AppState, env: &Env) {
        child.event(ctx, event, data, env);
        if let Event::KeyDown(_) = event {
            //TODO set some sort of debounce
            //TODO write the text_box to file
            println!("{}", &data.text_box);

            fs::write("./index.js", &data.text_box).expect("Unable to write file");
            let out = Command::new("node")
                .arg("./index.js")
                .output()
                .expect("node issues yer");

            let stdo = String::from_utf8_lossy(&out.stdout);
            println!("stdout: {}", stdo);
            data.output_box = stdo.to_string()

            // println!("{}", out.status);
            // println!("stderr: {}", String::from_utf8_lossy(&out.stderr));
        }
    }
}

fn main() {
    // describe the main window
    let main_window = WindowDesc::new(build_app)
        .title(WINDOW_TITLE)
        .window_size((400.0, 400.0));

    // create the initial app state
    let initial_state = AppState {
        import_box_chars: 0,
        import_box: "".to_string(),
        text_box: "".to_string(),
        output_box: "".to_string()
    };

    // start the application
    AppLauncher::with_window(main_window)
        .launch(initial_state)
        .expect("Failed to launch application");
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

    let right_column = Flex::column()
        .with_flex_child(output_box, 100.0)
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

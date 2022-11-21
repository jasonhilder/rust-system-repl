use crate::{
    docker_coms,
    RsrEvent
};
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
    Color,
    widget:: {
        Align,
        CrossAxisAlignment,
        Flex,
        TextBox,
        Spinner,
        Controller,
        Button,
        Label,
        Tabs
    }
};

const WINDOW_TITLE: LocalizedString<AppState> = LocalizedString::new("RJSI");
//const VERTICAL_WIDGET_SPACING: f64 = 20.0;

#[derive(Clone, Data, Lens)]
pub struct AppState {
    pub import_box_chars: u64,
    pub import_box: String,
    pub text_box: String,
    pub output_box: String,
    pub loading: bool,
    pub loading_msg: String,
    pub processing: bool,
    pub edited_timestamp: u64
}

struct Execute;

impl<W: Widget<AppState>> Controller<AppState, W> for Execute {
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
            if !data.processing {
                ctx.submit_command(docker_coms::submit_rsr_event(RsrEvent::Exec(data.text_box.clone())));
            }
        }
    }
}

fn build_app() -> impl Widget<AppState> {
    let show_logs = Button::from_label(Label::new("logs")
        .with_text_color(Color::grey(0.5)))
        .on_click(|_ctx, data: &mut AppState, _env| {});

    let import_btn = Button::from_label(Label::new("Import Libraries")
        .with_text_color(Color::grey(0.5)))
        .expand_width()
        .on_click(|ctx, data: &mut AppState, _env| {
            ctx.submit_command(
                docker_coms::submit_rsr_event(RsrEvent::ImportLibs(data.import_box.clone()))
            );
        });

    let imports_box = TextBox::multiline()
        .with_placeholder("Npm Imports")
        .expand_width()
        .expand_height()
        .lens(AppState::import_box);

    let import_container = Flex::column()
        .with_flex_child(imports_box, 95.0)
        .with_default_spacer()
        .with_flex_child(import_btn, 5.0)
        .expand_width();

    let text_box = TextBox::multiline()
        .with_placeholder("Code here")
        .expand_width()
        .expand_height()
        .lens(AppState::text_box)
        .controller(Execute);

    let text_tabs = Tabs::new()
        .with_tab("Code", text_box)
        .with_tab("Libs", import_container);

    let loading = Spinner::new()
        .expand_width()
        .expand_height();

    let detail_box = TextBox::new()
        .with_placeholder("Output")
        .expand_width()
        .expand_height()
        .lens(AppState::loading_msg);

    let output_box = TextBox::multiline()
        .with_placeholder("Output")
        .expand_width()
        .expand_height()
        .lens(AppState::output_box);

    let left_column = Flex::column()
        .with_flex_child(text_tabs, 100.0)
        .padding(5.0)
        .expand_height()
        .expand_width();

    let right_column = Flex::column()
        .with_flex_child(output_box, 100.0)
        .padding(5.0)
        .expand_height()
        .expand_width();

    let bottom_full_width_col = Flex::row()
        .with_flex_child(loading, 3.0)
        .with_default_spacer()
        .with_flex_child(detail_box, 90.0)
        .with_default_spacer()
        .with_flex_child(show_logs, 7.0)
        .padding(5.0)
        .expand_width();

    let top_row = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .must_fill_main_axis(true)
        .with_flex_child(left_column, 50.0)
        .with_default_spacer()
        .with_flex_child(right_column, 50.0)
        .expand_width();

    let bottom_row = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_flex_child(bottom_full_width_col, 100.0)
        .expand_width();

    let container = Flex::column()
        .with_flex_child(top_row, 90.0)
        .with_flex_child(bottom_row, 10.0)
        .expand_width();

    // center the two widgets in the available space
    Align::centered(container)
}

pub fn build_window() -> WindowDesc<AppState> {
    WindowDesc::new(build_app)
            .title(WINDOW_TITLE)
            .window_size((800.0, 500.0))
}

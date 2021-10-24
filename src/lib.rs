mod gamepad;
mod config;

use log::{error, info, warn};
use obs_wrapper::{
    graphics::*, log::Logger, obs_register_module, obs_string, prelude::*, source::*,
};
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Transform};

obs_register_module!(GamepadModule);
struct GamepadModule {
    context: ModuleContext,
}

struct Source;

struct State {
    pub img: Pixmap,
    pub texture: GraphicsTexture,
    pub width: u32,
    pub height: u32,
    pub color: Color,
}

impl Sourceable for Source {
    fn get_id() -> ObsString {
        obs_string!("gamepad")
    }

    fn get_type() -> SourceType {
        SourceType::INPUT
    }
}

const SETTING_WIDTH: ObsString = obs_string!("width");
const SETTING_HEIGHT: ObsString = obs_string!("height");
const SETTING_GAMEPAD: ObsString = obs_string!("gamepad");
const SETTING_FILE: ObsString = obs_string!("data file");

impl CreatableSource<State> for Source {
    fn create(ctx: &mut CreatableSourceContext<State>, _source: SourceContext) -> State {
        // let settings = &ctx.settings;
        ctx.register_hotkey(
            obs_string!("next gamepad"),
            obs_string!("switch to a different gamepad"),
            |key: &mut hotkey::Hotkey, state: &mut Option<State>| {
                if let Some(state) = state {
                    if key.pressed {
                        state.color = Color::from_rgba8(127, 150, 50, 200);
                    } else {
                        state.color = Color::from_rgba8(150, 50, 127, 200);
                    }
                }
                info!("Pressed button: {} {}", key.pressed, key.id());
            },
        );
        // TODO: settings width/height
        let width = 500; // source.get_base_width
        let height = 500;
        let color = Color::from_rgba8(50, 127, 150, 200);
        info!("Created gamepad source");
        State {
            img: Pixmap::new(width, height).unwrap(),
            texture: GraphicsTexture::new(width, height, GraphicsColorFormat::RGBA),
            width,
            height,
            color,
        }
    }
}

impl GetPropertiesSource<State> for Source {
    fn get_properties(state: &mut Option<State>, properties: &mut Properties) {
        todo!()
    }
}

impl GetDefaultsSource<State> for Source {
    fn get_defaults(_settings: &mut DataObj) {
        todo!("need to add obs_data_set_default_type")
    }
}

impl UpdateSource<State> for Source {
    fn update(
        _state: &mut Option<State>,
        _settings: &mut DataObj,
        _context: &mut GlobalContext,
    ) {
        todo!("settings update")
    }
}

impl GetNameSource<State> for Source {
    fn get_name() -> ObsString {
        obs_string!("Gamepad")
    }
}

impl GetWidthSource<State> for Source {
    fn get_width(state: &mut Option<State>) -> u32 {
        state.as_ref().map(|s| s.img.width()).unwrap()
    }
}

impl GetHeightSource<State> for Source {
    fn get_height(state: &mut Option<State>) -> u32 {
        state.as_ref().map(|s| s.img.height()).unwrap()
    }
}

impl VideoRenderSource<State> for Source {
    fn video_render(
        state: &mut Option<State>,
        _ctx: &mut GlobalContext,
        _vid_ctx: &mut VideoRenderContext,
    ) {
        if let Some(state) = state {
            state.img.fill(Color::from_rgba8(255, 0, 255, 64));

            let mut paint = Paint::default();
            paint.set_color(state.color);
            state.img.fill_path(
                &PathBuilder::from_circle(250.0, 250.0, 32.0).unwrap(),
                &paint,
                FillRule::default(),
                Transform::default(),
                None,
            );

            state.texture.set_image(
                state.img.data(),
                state.width * 4, // line size in bytes
                false,
            );
            state.texture.draw(0, 0, state.width, state.height, false);
        }
    }
}

impl Module for GamepadModule {
    fn new(context: ModuleContext) -> Self {
        Self { context }
    }

    fn get_ctx(&self) -> &ModuleContext {
        &self.context
    }

    fn load(&mut self, load_context: &mut LoadContext) -> bool {
        let source = load_context
            .create_source_builder::<Source, State>()
            .enable_create()
            // .enable_activate()
            // .enable_deactivate()
            .enable_get_name()
            .enable_get_width()
            .enable_get_height()
            // .enable_get_properties()
            // .enable_get_defaults()
            // .enable_update()
            .enable_video_render()
            .build();

        load_context.register_source(source);
        Logger::new().with_promote_debug(true).init().is_ok()
    }

    fn description() -> ObsString {
        obs_string!("A simple visualizer for gamepads")
    }

    fn name() -> ObsString {
        obs_string!("Gamepad Visualizer")
    }

    fn author() -> ObsString {
        obs_string!("Pineapple")
    }
}

mod config;
mod gamepad;

use std::{borrow::Cow, fs, path::PathBuf, time::Duration};

use gilrs_core::{self, Gilrs};
use log::{error, info};
use notify::{self, DebouncedEvent};
use obs_wrapper::{
    graphics::*, log::Logger, obs_register_module, obs_string, prelude::*, source::*,
};
use tiny_skia::Pixmap;

use config::ConfigWatcher;
use gamepad::Gamepad;

obs_register_module!(GamepadModule);
struct GamepadModule {
    context: ModuleContext,
}

struct Source;

impl Sourceable for Source {
    fn get_id() -> ObsString {
        obs_string!("gamepad")
    }

    fn get_type() -> SourceType {
        SourceType::INPUT
    }
}

pub struct Image {
    pub mine: Pixmap,
    pub obs: GraphicsTexture,
    pub width: u32,
    pub height: u32,
    pub force_render: bool,
}

impl From<&Gamepad> for Image {
    fn from(gamepad: &Gamepad) -> Self {
        let (width, height) = if gamepad.is_empty() {
            (100, 100)
        } else {
            let bounds = gamepad.bounds();
            (bounds.right() as u32, bounds.bottom() as u32)
        };
        Self {
            mine: Pixmap::new(width, height).unwrap(),
            obs: GraphicsTexture::new(width, height, GraphicsColorFormat::RGBA),
            width,
            height,
            force_render: true,
        }
    }
}

struct State {
    pub image: Image,
    pub gilrs: Gilrs,
    pub gamepad: Gamepad,
    pub watcher: ConfigWatcher,
}

impl State {
    fn update_config(&mut self) {
        info!("config update");
        let path = self.watcher.path.as_ref().unwrap();
        self.image.force_render = true;
        match toml::from_str(&fs::read_to_string(path).unwrap()) {
            Ok(config) => {
                self.gamepad.load_config(&mut self.gilrs, &config);
                let bounds = self.gamepad.bounds();
                if self.image.width != bounds.right() as u32
                    || self.image.height != bounds.bottom() as u32
                {
                    info!("resized image");
                    self.image = (&self.gamepad).into();
                }
            }
            Err(e) => {
                error!("Config reload failed: {}", e);
            }
        }
    }

    fn update_settings(&mut self, settings: &DataObj) {
        if let Some(id) = settings.get(SETTING_GAMEPAD) {
            self.gamepad.switch_gamepad(&mut self.gilrs, id);
        }
        if let Some(path) = settings.get::<Cow<str>, _>(SETTING_FILE) {
            let new = PathBuf::from(path.as_ref());
            if self.watcher.path.as_ref() != Some(&new) {
                self.watcher.change_file(new).unwrap();
                self.update_config();
            }
        }
    }
}

impl Drop for State {
    fn drop(&mut self) {
        info!("state destroyed")
    }
}

const SETTING_GAMEPAD: ObsString = obs_string!("gamepad");
const SETTING_FILE: ObsString = obs_string!("settings");

impl CreatableSource<State> for Source {
    fn create(ctx: &mut CreatableSourceContext<State>, _source: SourceContext) -> State {
        let gamepad = Gamepad::default();
        let gilrs = Gilrs::new().unwrap();
        let watcher = ConfigWatcher::new(Duration::from_millis(200));
        let mut state = State {
            image: (&gamepad).into(),
            gilrs,
            gamepad,
            watcher,
        };
        state.update_settings(&ctx.settings);
        info!("created gamepad source");
        state
    }
}

impl GetPropertiesSource<State> for Source {
    fn get_properties(state: &mut Option<State>, properties: &mut Properties) {
        if let Some(state) = state {
            let max_gamepads = state.gilrs.last_gamepad_hint();
            properties.add(
                SETTING_GAMEPAD,
                obs_string!("Gamepad ID"),
                NumberProp::new_int().with_range(0..max_gamepads),
            );
            properties.add(
                SETTING_FILE,
                obs_string!("Layout File"),
                PathProp::new(PathType::File),
            );
        }
    }
}

// TODO: https://github.com/bennetthardwick/rust-obs-plugins/pull/15
// default to last active gamepad and an xbox config file
impl GetDefaultsSource<State> for Source {
    fn get_defaults(_settings: &mut DataObj) {
        unimplemented!()
    }
}

impl UpdateSource<State> for Source {
    fn update(
        state: &mut Option<State>,
        settings: &mut DataObj,
        _context: &mut GlobalContext,
    ) {
        info!("settings update");
        if let Some(state) = state {
            state.update_settings(settings);
        }
    }
}

impl GetNameSource<State> for Source {
    fn get_name() -> ObsString {
        obs_string!("Gamepad")
    }
}

impl GetWidthSource<State> for Source {
    fn get_width(state: &mut Option<State>) -> u32 {
        state.as_ref().map(|s| s.image.width).unwrap()
    }
}

impl GetHeightSource<State> for Source {
    fn get_height(state: &mut Option<State>) -> u32 {
        state.as_ref().map(|s| s.image.height).unwrap()
    }
}

impl VideoRenderSource<State> for Source {
    fn video_render(
        state: &mut Option<State>,
        _ctx: &mut GlobalContext,
        _vid_ctx: &mut VideoRenderContext,
    ) {
        if let Some(state) = state {
            while let Ok(event) = state.watcher.rx.try_recv() {
                use DebouncedEvent::*;
                match event {
                    Create(p) | Write(p) => {
                        if state.watcher.path == Some(p) {
                            state.update_config()
                        }
                    }
                    _ => {}
                }
            }
            if state.gamepad.update(&mut state.gilrs) || state.image.force_render {
                state.image.force_render = false;
                state.gamepad.render(&mut state.image.mine);
                state.image.obs.set_image(
                    state.image.mine.data(),
                    state.image.width * 4, // line size in bytes
                    false,
                );
            }
            state
                .image
                .obs
                .draw(0, 0, state.image.width, state.image.height, false);
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
            .enable_get_name()
            .enable_get_width()
            .enable_get_height()
            .enable_get_properties()
            .enable_update()
            .enable_video_render()
            // .enable_get_defaults()
            // .enable_activate()
            // .enable_deactivate()
            .build();
        // TODO: set source icon_type

        load_context.register_source(source);
        Logger::new().init().is_ok()
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

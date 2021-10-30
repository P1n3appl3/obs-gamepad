mod config;
mod gamepad;

use std::{
    borrow::Cow,
    fs,
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    time::Duration,
};

use gilrs_core::{self, Gilrs};
use log::info;
use notify::{self, DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use obs_wrapper::{
    graphics::*, log::Logger, obs_register_module, obs_string, prelude::*, source::*,
};
use tiny_skia::Pixmap;

use gamepad::Gamepad;

obs_register_module!(GamepadModule);
struct GamepadModule {
    context: ModuleContext,
}

struct Source;

pub struct Image {
    pub mine: Pixmap,
    pub obs: GraphicsTexture,
    pub width: u32,
    pub height: u32,
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
        }
    }
}

pub struct ConfigWatcher {
    pub watcher: RecommendedWatcher,
    pub receiver: Receiver<DebouncedEvent>,
    pub path: Option<PathBuf>,
}

struct State {
    pub image: Image,
    pub gilrs: Gilrs,
    pub gamepad: Gamepad,
    pub watcher: ConfigWatcher,
}

impl Drop for State {
    fn drop(&mut self) {
        info!("state destroyed")
    }
}

impl Sourceable for Source {
    fn get_id() -> ObsString {
        obs_string!("gamepad")
    }

    fn get_type() -> SourceType {
        SourceType::INPUT
    }
}

const SETTING_GAMEPAD: ObsString = obs_string!("gamepad");
const SETTING_FILE: ObsString = obs_string!("settings");

impl CreatableSource<State> for Source {
    fn create(ctx: &mut CreatableSourceContext<State>, _source: SourceContext) -> State {
        ctx.register_hotkey(
            obs_string!("next gamepad"),
            obs_string!("switch to a different gamepad"),
            |key, state| {
                if let Some(_state) = state {
                    if key.pressed {
                        // TODO:
                    }
                }
                info!(
                    "{} button: {}",
                    if key.pressed { "pressed" } else { "released" },
                    key.id()
                );
            },
        );
        let settings = &ctx.settings;
        let mut gamepad = Gamepad {
            id: settings.get(SETTING_GAMEPAD).unwrap_or_default(),
            ..Default::default()
        };
        let (tx, rx) = mpsc::channel();
        let mut watcher = ConfigWatcher {
            watcher: RecommendedWatcher::new(tx, Duration::from_millis(100)).unwrap(),
            receiver: rx,
            path: None,
        };
        let mut gilrs = Gilrs::new().unwrap();
        if let Some(path) = settings.get::<Cow<str>, _>(SETTING_FILE) {
            let path = PathBuf::from(path.as_ref());
            let data = &toml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
            gamepad.load_config(&mut gilrs, &data);
            watcher
                .watcher
                .watch(path.parent().unwrap(), RecursiveMode::Recursive)
                .unwrap();
            watcher.path = Some(path);
        }
        info!("Created gamepad source");
        State {
            image: (&gamepad).into(),
            gilrs,
            gamepad,
            watcher,
        }
    }
}

impl GetPropertiesSource<State> for Source {
    fn get_properties(state: &mut Option<State>, properties: &mut Properties) {
        info!("loading props");
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
impl GetDefaultsSource<State> for Source {
    fn get_defaults(_settings: &mut DataObj) {
        // TODO: last active gamepad, and xbox default file
        unimplemented!()
    }
}

impl UpdateSource<State> for Source {
    fn update(
        _state: &mut Option<State>,
        _settings: &mut DataObj,
        _context: &mut GlobalContext,
    ) {
        info!("settings update");
        // todo!("settings update")
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
        if let Some(State {
            image,
            gamepad,
            gilrs,
            ..
        }) = state
        {
            gamepad.update(gilrs);
            gamepad.render(&mut image.mine);
            image.obs.set_image(
                image.mine.data(),
                image.width * 4, // line size in bytes
                false,
            );
            image.obs.draw(0, 0, image.width, image.height, false);
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

mod config;
mod gamepad;
mod serial;
mod usb;

use std::{
    borrow::Cow,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use gilrs_core::{self, Gilrs};
use log::{error, info};
use notify_debouncer_mini::{DebouncedEvent, DebouncedEventKind};
use obs_wrapper::{
    graphics::*, log::Logger, obs_register_module, obs_string, prelude::*, properties::*,
    source::*,
};
use tiny_skia::Pixmap;

use config::ConfigWatcher;
use gamepad::{Gamepad, Inputs};
use usb::UsbGamepad;

obs_register_module!(GamepadModule);
struct GamepadModule {
    context: ModuleContext,
}

struct Source<'b> {
    pub image: Image,
    pub gamepad: Gamepad<'b>,
    pub watcher: ConfigWatcher,
    pub device_id: usize,
}

pub struct Image {
    pub mine: Pixmap,
    pub obs: GraphicsTexture,
    pub width: u32,
    pub height: u32,
    pub force_render: bool,
}

impl From<&Inputs> for Image {
    fn from(inputs: &Inputs) -> Self {
        let (width, height) = if inputs.buttons.is_empty()
            && inputs.axes.is_empty()
            && inputs.sticks.is_empty()
        {
            (100, 100)
        } else {
            let bounds = inputs.bounds();
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

impl<'b> Source<'b> {
    fn update_config(&mut self, path: &Path) {
        info!("config update");
        self.image.force_render = true;
        match toml::from_str(&fs::read_to_string(path).unwrap()) {
            Ok(config) => {
                self.gamepad
                    .load::<UsbGamepad>(&config, (Gilrs::new().unwrap(), self.device_id))
                    .ok();
                let bounds = self.gamepad.inputs.bounds();
                if self.image.width != bounds.right() as u32
                    || self.image.height != bounds.bottom() as u32
                {
                    self.image = (&self.gamepad.inputs).into();
                }
            }
            Err(e) => {
                error!("Config reload failed: {}", e);
            }
        }
    }

    fn update_settings(&mut self, settings: &DataObj) {
        if let Some(id) = settings.get(SETTING_GAMEPAD) {
            self.device_id = id;
            if let Some(p) = self.watcher.path.clone() {
                self.update_config(&p)
            }
        }
        if let Some(path) = settings.get::<Cow<str>>(SETTING_FILE) {
            let new = PathBuf::from(path.as_ref());
            if self.watcher.path.as_ref() != Some(&new) {
                self.update_config(&new);
                self.watcher.change_file(new).unwrap();
            }
        }
    }
}

const SETTING_GAMEPAD: ObsString = obs_string!("gamepad");
const SETTING_FILE: ObsString = obs_string!("settings");

impl<'b> Sourceable for Source<'b> {
    fn create(
        ctx: &mut CreatableSourceContext<Source>,
        _source: SourceContext,
    ) -> Source<'b> {
        let gamepad = Gamepad::default();
        let watcher = ConfigWatcher::new(Duration::from_millis(200));
        let mut source =
            Source { image: (&gamepad.inputs).into(), gamepad, watcher, device_id: 0 };
        source.update_settings(&ctx.settings);
        source
    }

    fn get_id() -> ObsString {
        obs_string!("gamepad")
    }

    fn get_type() -> SourceType {
        SourceType::INPUT
    }
}

impl GetPropertiesSource for Source<'_> {
    fn get_properties(&mut self) -> Properties {
        // TODO: re-scan upon opening properties? this could be handled better
        // TODO: show list including gamepads, serial ports, etc.
        let max_gamepads = Gilrs::new().unwrap().last_gamepad_hint();
        let mut props = Properties::new();
        props.add(
            SETTING_GAMEPAD,
            obs_string!("Gamepad ID"),
            NumberProp::new_int().with_range(0..max_gamepads.max(1)),
        );
        let path_config = PathProp::new(PathType::File)
            .with_filter(obs_string!("TOML config file (*.toml)"));
        // TODO: with_default_path pointing to the example.toml in the config dir
        props.add(SETTING_FILE, obs_string!("Layout File"), path_config);
        props
    }
}

impl UpdateSource for Source<'_> {
    fn update(&mut self, settings: &mut DataObj, _context: &mut GlobalContext) {
        self.update_settings(settings);
    }
}

impl GetNameSource for Source<'_> {
    fn get_name() -> ObsString {
        obs_string!("Gamepad")
    }
}

impl GetWidthSource for Source<'_> {
    fn get_width(&mut self) -> u32 {
        self.image.width
    }
}

impl GetHeightSource for Source<'_> {
    fn get_height(&mut self) -> u32 {
        self.image.height
    }
}

impl VideoRenderSource for Source<'_> {
    fn video_render(
        &mut self,
        _ctx: &mut GlobalContext,
        _vid_ctx: &mut VideoRenderContext,
    ) {
        while let Ok(DebouncedEvent { path, kind: DebouncedEventKind::Any }) =
            self.watcher.rx.try_recv()
        {
            if self.watcher.path.as_deref() == Some(&path) {
                self.update_config(&path)
            }
        }
        if self.gamepad.poll() || self.image.force_render {
            self.image.force_render = false;
            self.gamepad.render(&mut self.image.mine);
            self.image.obs.set_image(
                self.image.mine.data(),
                self.image.width * 4, // line size in bytes
                false,
            );
        }
        self.image.obs.draw(0, 0, self.image.width, self.image.height, false);
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
        let source_info = load_context
            .create_source_builder::<Source<'_>>()
            .enable_get_name()
            .enable_get_width()
            .enable_get_height()
            .enable_get_properties()
            .enable_update()
            .enable_video_render()
            .with_icon(Icon::GameCapture)
            .build();
        load_context.register_source(source_info);
        Logger::new().init().is_ok()
    }

    fn description() -> ObsString {
        obs_string!("A simple visualizer for gamepads")
    }

    fn name() -> ObsString {
        obs_string!("Gamepad Visualizer")
    }

    fn author() -> ObsString {
        obs_string!("pineapple")
    }
}

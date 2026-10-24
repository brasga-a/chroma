//! Deterministic, transport-independent keyboard lighting composition.
//!
//! The crate owns no clock, thread, device handle, or operating-system API.
//! A caller supplies absolute millisecond timestamps to [`Engine::render_at`].

use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::fmt;

pub const PROTOCOL_VERSION: u32 = 1;
pub const MAX_DIMENSION: u16 = 64;
pub const MAX_CELLS: usize = 4096;
pub const MAX_LEDS: usize = 1024;
pub const MAX_LAYERS: usize = 32;
pub const MAX_EVENTS: usize = 512;
pub const DEFAULT_EVENT_CAPACITY: usize = 256;
pub const MAX_ID_BYTES: usize = 128;
pub const MAX_GRADIENT_COLORS: usize = 16;
pub const MAX_EFFECT_DURATION_MS: u64 = 31_536_000_000;
pub const MAX_JSON_LINE_BYTES: usize = 1_048_576;

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct KeyId(pub String);

impl From<&str> for KeyId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for KeyId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LedId(pub String);

impl From<&str> for LedId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for LedId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgba {
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const BLACK: Self = Self { r: 0, g: 0, b: 0 };

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeyPosition {
    pub id: KeyId,
    pub x: u16,
    pub y: u16,
}

impl KeyPosition {
    pub fn new(id: impl Into<KeyId>, x: u16, y: u16) -> Self {
        Self {
            id: id.into(),
            x,
            y,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LedPosition {
    pub id: LedId,
    pub key_id: Option<KeyId>,
    pub x: u16,
    pub y: u16,
}

impl LedPosition {
    pub fn new(id: impl Into<LedId>, key_id: Option<KeyId>, x: u16, y: u16) -> Self {
        Self {
            id: id.into(),
            key_id,
            x,
            y,
        }
    }
}

/// A configurable two-dimensional geometry. Missing LED coordinates are holes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    pub id: String,
    pub width: u16,
    pub height: u16,
    pub keys: Vec<KeyPosition>,
    pub leds: Vec<LedPosition>,
}

impl Layout {
    pub fn new(
        id: impl Into<String>,
        width: u16,
        height: u16,
        keys: Vec<KeyPosition>,
        leds: Vec<LedPosition>,
    ) -> Self {
        Self {
            id: id.into(),
            width,
            height,
            keys,
            leds,
        }
    }

    /// A synthetic 6×22 matrix for geometry and protocol examples.
    ///
    /// The IDs are generated placeholders. This does not claim a physical
    /// key/LED mapping for any particular device.
    pub fn geometric_6x22_template() -> Self {
        let mut keys = Vec::with_capacity(6 * 22);
        let mut leds = Vec::with_capacity(6 * 22);
        for y in 0..6 {
            for x in 0..22 {
                let suffix = format!("r{y}c{x}");
                let key_id = KeyId(format!("template-key-{suffix}"));
                keys.push(KeyPosition::new(key_id.clone(), x, y));
                leds.push(LedPosition::new(
                    format!("template-led-{suffix}"),
                    Some(key_id),
                    x,
                    y,
                ));
            }
        }
        Self::new("geometric-matrix-6x22-template", 22, 6, keys, leds)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlendMode {
    /// Standard source-over coverage: sample alpha × layer opacity.
    Over,
    /// Adds sample RGB weighted by sample alpha × layer opacity, clamped to 1.
    Add,
    /// Multiplies destination RGB toward sample RGB by sample alpha × opacity.
    Multiply,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaveDirection {
    Horizontal,
    Vertical,
    Diagonal,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Effect {
    Static {
        color: Rgba,
    },
    Breathing {
        color: Rgba,
        period_ms: u64,
    },
    Spectrum {
        period_ms: u64,
        wavelength_cells: f32,
    },
    Wave {
        colors: Vec<Rgba>,
        speed_cells_per_second: f32,
        wavelength_cells: f32,
        direction: WaveDirection,
    },
    Reactive {
        color: Rgba,
        fade_ms: u64,
    },
    Ripple {
        colors: Vec<Rgba>,
        duration_ms: u64,
        speed_cells_per_second: f32,
        width_cells: f32,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layer {
    pub id: String,
    pub enabled: bool,
    pub opacity: f32,
    /// `None` selects all keys; an empty set selects none.
    pub mask: Option<BTreeSet<KeyId>>,
    pub blend: BlendMode,
    pub effect: Effect,
}

impl Layer {
    pub fn new(id: impl Into<String>, effect: Effect) -> Self {
        Self {
            id: id.into(),
            enabled: true,
            opacity: 1.0,
            mask: None,
            blend: BlendMode::Over,
            effect,
        }
    }
}

/// Layers are ordered from bottom to top.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    pub id: String,
    pub layers: Vec<Layer>,
}

impl Scene {
    pub fn new(id: impl Into<String>, layers: Vec<Layer>) -> Self {
        Self {
            id: id.into(),
            layers,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrameCell {
    pub led_id: LedId,
    pub key_id: Option<KeyId>,
    pub rgb: Rgb,
}

/// A row-major RGB matrix. `None` entries are physical/layout holes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    pub layout_id: String,
    pub width: u16,
    pub height: u16,
    pub time_ms: u64,
    pub cells: Vec<Option<FrameCell>>,
}

impl Frame {
    pub fn cell_at(&self, x: u16, y: u16) -> Option<&FrameCell> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let index = usize::from(y) * usize::from(self.width) + usize::from(x);
        self.cells.get(index)?.as_ref()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    SetLayout { layout: Layout },
    SetScene { scene: Scene },
    SetConfiguration { layout: Layout, scene: Scene },
    KeyPress { key_id: KeyId, time_ms: u64 },
    RenderAt { time_ms: u64 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub version: u32,
    pub command: Command,
}

impl Request {
    pub fn new(command: Command) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            command,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommandResult {
    LayoutSet { layout_id: String },
    SceneSet { scene_id: String },
    ConfigurationSet { layout_id: String, scene_id: String },
    KeyPressAccepted { retained_events: usize },
    Frame { frame: Frame },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineError {
    pub code: String,
    pub message: String,
}

impl EngineError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for EngineError {}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub version: u32,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<CommandResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<EngineError>,
}

impl Response {
    pub fn success(result: CommandResult) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            ok: true,
            result: Some(result),
            error: None,
        }
    }

    pub fn failure(error: EngineError) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            ok: false,
            result: None,
            error: Some(error),
        }
    }
}

#[derive(Clone, Debug)]
struct KeyEvent {
    key_id: KeyId,
    time_ms: u64,
}

/// Validated scene state plus a bounded event history. It has no internal clock.
#[derive(Clone, Debug)]
pub struct Engine {
    layout: Layout,
    scene: Scene,
    key_positions: HashMap<KeyId, (u16, u16)>,
    events: VecDeque<KeyEvent>,
    event_capacity: usize,
}

impl Engine {
    pub fn new(layout: Layout, scene: Scene) -> Result<Self, EngineError> {
        Self::with_event_capacity(layout, scene, DEFAULT_EVENT_CAPACITY)
    }

    pub fn with_event_capacity(
        layout: Layout,
        scene: Scene,
        event_capacity: usize,
    ) -> Result<Self, EngineError> {
        if !(1..=MAX_EVENTS).contains(&event_capacity) {
            return Err(EngineError::new(
                "invalid_event_capacity",
                format!("event capacity must be between 1 and {MAX_EVENTS}"),
            ));
        }
        validate_layout(&layout)?;
        validate_scene(&scene, &layout)?;
        let key_positions = layout
            .keys
            .iter()
            .map(|key| (key.id.clone(), (key.x, key.y)))
            .collect();
        Ok(Self {
            layout,
            scene,
            key_positions,
            events: VecDeque::with_capacity(event_capacity),
            event_capacity,
        })
    }

    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    pub fn retained_event_count(&self) -> usize {
        self.events.len()
    }

    /// Validate the candidate layout and scene compatibility first. A successful
    /// replacement clears old events because their IDs and coordinates belong to
    /// the previous layout; a rejected replacement leaves the history intact.
    pub fn set_layout(&mut self, layout: Layout) -> Result<(), EngineError> {
        validate_layout(&layout)?;
        validate_scene(&self.scene, &layout)?;
        let key_positions = layout
            .keys
            .iter()
            .map(|key| (key.id.clone(), (key.x, key.y)))
            .collect();
        self.layout = layout;
        self.key_positions = key_positions;
        self.events.clear();
        Ok(())
    }

    /// Replace the scene only after every layer and parameter has passed validation.
    pub fn set_scene(&mut self, scene: Scene) -> Result<(), EngineError> {
        validate_scene(&scene, &self.layout)?;
        self.scene = scene;
        Ok(())
    }

    /// Atomically replace both layout and scene after validating them together.
    /// A successful configuration change clears events tied to the old layout.
    pub fn set_configuration(&mut self, layout: Layout, scene: Scene) -> Result<(), EngineError> {
        validate_layout(&layout)?;
        validate_scene(&scene, &layout)?;
        let key_positions = layout
            .keys
            .iter()
            .map(|key| (key.id.clone(), (key.x, key.y)))
            .collect();
        self.layout = layout;
        self.scene = scene;
        self.key_positions = key_positions;
        self.events.clear();
        Ok(())
    }

    /// Record a key press in nondecreasing event-time order.
    ///
    /// When at capacity, the oldest event is evicted. Invalid IDs or out-of-order
    /// timestamps leave the event history unchanged.
    pub fn key_press(&mut self, key_id: KeyId, time_ms: u64) -> Result<usize, EngineError> {
        if !self.key_positions.contains_key(&key_id) {
            return Err(EngineError::new(
                "unknown_key",
                format!(
                    "key ID '{}' is not present in layout '{}'",
                    key_id.0, self.layout.id
                ),
            ));
        }
        if let Some(last) = self.events.back()
            && time_ms < last.time_ms
        {
            return Err(EngineError::new(
                "out_of_order_event",
                format!(
                    "key press time {time_ms} is earlier than the most recent event at {}",
                    last.time_ms
                ),
            ));
        }
        if self.events.len() == self.event_capacity {
            self.events.pop_front();
        }
        self.events.push_back(KeyEvent { key_id, time_ms });
        Ok(self.events.len())
    }

    /// Render from absolute time. This method is pure with respect to engine state.
    pub fn render_at(&self, time_ms: u64) -> Frame {
        let width = usize::from(self.layout.width);
        let height = usize::from(self.layout.height);
        let mut cells = vec![None; width * height];

        for led in &self.layout.leds {
            let mut rgb = [0.0_f32; 3];
            for layer in &self.scene.layers {
                if !layer.enabled || layer.opacity == 0.0 {
                    continue;
                }
                if let Some(mask) = &layer.mask
                    && !led
                        .key_id
                        .as_ref()
                        .is_some_and(|key_id| mask.contains(key_id))
                {
                    continue;
                }
                let Some(sample) = sample_effect(
                    &layer.effect,
                    led,
                    time_ms,
                    &self.events,
                    &self.key_positions,
                ) else {
                    continue;
                };
                let coverage = sample.a * layer.opacity;
                if coverage <= 0.0 {
                    continue;
                }
                apply_blend(&mut rgb, sample, coverage, layer.blend);
            }

            let index = usize::from(led.y) * width + usize::from(led.x);
            cells[index] = Some(FrameCell {
                led_id: led.id.clone(),
                key_id: led.key_id.clone(),
                rgb: Rgb::new(to_channel(rgb[0]), to_channel(rgb[1]), to_channel(rgb[2])),
            });
        }

        Frame {
            layout_id: self.layout.id.clone(),
            width: self.layout.width,
            height: self.layout.height,
            time_ms,
            cells,
        }
    }

    pub fn execute(&mut self, command: Command) -> Result<CommandResult, EngineError> {
        match command {
            Command::SetLayout { layout } => {
                let layout_id = layout.id.clone();
                self.set_layout(layout)?;
                Ok(CommandResult::LayoutSet { layout_id })
            }
            Command::SetScene { scene } => {
                let scene_id = scene.id.clone();
                self.set_scene(scene)?;
                Ok(CommandResult::SceneSet { scene_id })
            }
            Command::SetConfiguration { layout, scene } => {
                let layout_id = layout.id.clone();
                let scene_id = scene.id.clone();
                self.set_configuration(layout, scene)?;
                Ok(CommandResult::ConfigurationSet {
                    layout_id,
                    scene_id,
                })
            }
            Command::KeyPress { key_id, time_ms } => {
                let retained_events = self.key_press(key_id, time_ms)?;
                Ok(CommandResult::KeyPressAccepted { retained_events })
            }
            Command::RenderAt { time_ms } => Ok(CommandResult::Frame {
                frame: self.render_at(time_ms),
            }),
        }
    }

    pub fn handle_request(&mut self, request: Request) -> Response {
        if request.version != PROTOCOL_VERSION {
            return Response::failure(EngineError::new(
                "unsupported_version",
                format!(
                    "request version {} is unsupported; expected {PROTOCOL_VERSION}",
                    request.version
                ),
            ));
        }
        match self.execute(request.command) {
            Ok(result) => Response::success(result),
            Err(error) => Response::failure(error),
        }
    }
}

fn validate_layout(layout: &Layout) -> Result<(), EngineError> {
    validate_id("layout", &layout.id)?;
    if layout.width == 0 || layout.height == 0 {
        return Err(EngineError::new(
            "invalid_dimensions",
            "layout width and height must both be greater than zero",
        ));
    }
    if layout.width > MAX_DIMENSION || layout.height > MAX_DIMENSION {
        return Err(EngineError::new(
            "invalid_dimensions",
            format!("layout dimensions may not exceed {MAX_DIMENSION} on either axis"),
        ));
    }
    let cell_count = usize::from(layout.width)
        .checked_mul(usize::from(layout.height))
        .ok_or_else(|| EngineError::new("invalid_dimensions", "layout area overflowed"))?;
    if cell_count > MAX_CELLS {
        return Err(EngineError::new(
            "invalid_dimensions",
            format!("layout may not contain more than {MAX_CELLS} matrix cells"),
        ));
    }
    if layout.keys.len() > cell_count {
        return Err(EngineError::new(
            "too_many_keys",
            "layout has more keys than matrix coordinates",
        ));
    }
    if layout.leds.len() > MAX_LEDS || layout.leds.len() > cell_count {
        return Err(EngineError::new(
            "too_many_leds",
            format!("layout may contain at most {MAX_LEDS} LEDs and one LED per matrix cell"),
        ));
    }

    let mut keys = HashSet::with_capacity(layout.keys.len());
    let mut key_coordinates = HashSet::with_capacity(layout.keys.len());
    for key in &layout.keys {
        validate_id("key", &key.id.0)?;
        if key.x >= layout.width || key.y >= layout.height {
            return Err(EngineError::new(
                "invalid_coordinate",
                format!("key '{}' is outside the layout matrix", key.id.0),
            ));
        }
        if !keys.insert(key.id.clone()) {
            return Err(EngineError::new(
                "duplicate_id",
                format!("duplicate key ID '{}'", key.id.0),
            ));
        }
        if !key_coordinates.insert((key.x, key.y)) {
            return Err(EngineError::new(
                "duplicate_coordinate",
                format!("multiple keys occupy ({}, {})", key.x, key.y),
            ));
        }
    }

    let mut leds = HashSet::with_capacity(layout.leds.len());
    let mut led_coordinates = HashSet::with_capacity(layout.leds.len());
    for led in &layout.leds {
        validate_id("LED", &led.id.0)?;
        if led.x >= layout.width || led.y >= layout.height {
            return Err(EngineError::new(
                "invalid_coordinate",
                format!("LED '{}' is outside the layout matrix", led.id.0),
            ));
        }
        if !leds.insert(led.id.clone()) {
            return Err(EngineError::new(
                "duplicate_id",
                format!("duplicate LED ID '{}'", led.id.0),
            ));
        }
        if !led_coordinates.insert((led.x, led.y)) {
            return Err(EngineError::new(
                "duplicate_coordinate",
                format!("multiple LEDs occupy ({}, {})", led.x, led.y),
            ));
        }
        if let Some(key_id) = &led.key_id
            && !keys.contains(key_id)
        {
            return Err(EngineError::new(
                "unknown_key",
                format!("LED '{}' refers to missing key ID '{}'", led.id.0, key_id.0),
            ));
        }
    }
    Ok(())
}

fn validate_scene(scene: &Scene, layout: &Layout) -> Result<(), EngineError> {
    validate_id("scene", &scene.id)?;
    if scene.layers.len() > MAX_LAYERS {
        return Err(EngineError::new(
            "too_many_layers",
            format!("scene may contain at most {MAX_LAYERS} layers"),
        ));
    }
    let key_ids: HashSet<&KeyId> = layout.keys.iter().map(|key| &key.id).collect();
    let mut layer_ids = HashSet::with_capacity(scene.layers.len());
    for layer in &scene.layers {
        validate_id("layer", &layer.id)?;
        if !layer_ids.insert(layer.id.as_str()) {
            return Err(EngineError::new(
                "duplicate_id",
                format!("duplicate layer ID '{}'", layer.id),
            ));
        }
        if !layer.opacity.is_finite() || !(0.0..=1.0).contains(&layer.opacity) {
            return Err(EngineError::new(
                "invalid_opacity",
                format!(
                    "layer '{}' opacity must be finite and between 0 and 1",
                    layer.id
                ),
            ));
        }
        if let Some(mask) = &layer.mask {
            for key_id in mask {
                if !key_ids.contains(key_id) {
                    return Err(EngineError::new(
                        "unknown_key",
                        format!(
                            "layer '{}' mask refers to missing key ID '{}'",
                            layer.id, key_id.0
                        ),
                    ));
                }
            }
        }
        validate_effect(&layer.effect, &layer.id)?;
    }
    Ok(())
}

fn validate_effect(effect: &Effect, layer_id: &str) -> Result<(), EngineError> {
    match effect {
        Effect::Static { color } => validate_color(*color, layer_id),
        Effect::Breathing { color, period_ms } => {
            validate_color(*color, layer_id)?;
            validate_duration(*period_ms, "breathing period", layer_id)
        }
        Effect::Spectrum {
            period_ms,
            wavelength_cells,
        } => {
            validate_duration(*period_ms, "spectrum period", layer_id)?;
            validate_wavelength(*wavelength_cells, layer_id)
        }
        Effect::Wave {
            colors,
            speed_cells_per_second,
            wavelength_cells,
            ..
        } => {
            validate_gradient(colors, 2, layer_id)?;
            validate_speed(*speed_cells_per_second, true, layer_id)?;
            validate_wavelength(*wavelength_cells, layer_id)
        }
        Effect::Reactive { color, fade_ms } => {
            validate_color(*color, layer_id)?;
            validate_duration(*fade_ms, "reactive fade", layer_id)
        }
        Effect::Ripple {
            colors,
            duration_ms,
            speed_cells_per_second,
            width_cells,
        } => {
            validate_gradient(colors, 1, layer_id)?;
            validate_duration(*duration_ms, "ripple duration", layer_id)?;
            validate_speed(*speed_cells_per_second, false, layer_id)?;
            if !width_cells.is_finite() || !(0.01..=64.0).contains(width_cells) {
                return Err(EngineError::new(
                    "invalid_effect_parameter",
                    format!("layer '{layer_id}' ripple width must be between 0.01 and 64 cells"),
                ));
            }
            Ok(())
        }
    }
}

fn validate_id(kind: &str, id: &str) -> Result<(), EngineError> {
    if id.is_empty() || id.trim() != id || id.chars().any(char::is_control) {
        return Err(EngineError::new(
            "invalid_id",
            format!("{kind} ID must be nonempty, trimmed, and contain no control characters"),
        ));
    }
    if id.len() > MAX_ID_BYTES {
        return Err(EngineError::new(
            "invalid_id",
            format!("{kind} ID may not exceed {MAX_ID_BYTES} UTF-8 bytes"),
        ));
    }
    Ok(())
}

fn validate_color(color: Rgba, layer_id: &str) -> Result<(), EngineError> {
    for (name, channel) in [
        ("r", color.r),
        ("g", color.g),
        ("b", color.b),
        ("a", color.a),
    ] {
        if !channel.is_finite() || !(0.0..=1.0).contains(&channel) {
            return Err(EngineError::new(
                "invalid_color",
                format!(
                    "layer '{layer_id}' color channel {name} must be finite and between 0 and 1"
                ),
            ));
        }
    }
    Ok(())
}

fn validate_duration(duration_ms: u64, name: &str, layer_id: &str) -> Result<(), EngineError> {
    if duration_ms == 0 || duration_ms > MAX_EFFECT_DURATION_MS {
        return Err(EngineError::new(
            "invalid_effect_parameter",
            format!("layer '{layer_id}' {name} must be between 1 and {MAX_EFFECT_DURATION_MS} ms"),
        ));
    }
    Ok(())
}

fn validate_wavelength(wavelength_cells: f32, layer_id: &str) -> Result<(), EngineError> {
    if !wavelength_cells.is_finite() || !(0.01..=4096.0).contains(&wavelength_cells) {
        return Err(EngineError::new(
            "invalid_effect_parameter",
            format!("layer '{layer_id}' wavelength must be finite and between 0.01 and 4096 cells"),
        ));
    }
    Ok(())
}

fn validate_speed(speed: f32, allow_negative: bool, layer_id: &str) -> Result<(), EngineError> {
    let valid = speed.is_finite()
        && if allow_negative {
            (-1000.0..=1000.0).contains(&speed)
        } else {
            (0.0..=1000.0).contains(&speed) && speed > 0.0
        };
    if !valid {
        let allowed = if allow_negative {
            "-1000..=1000"
        } else {
            "greater than 0 and at most 1000"
        };
        return Err(EngineError::new(
            "invalid_effect_parameter",
            format!("layer '{layer_id}' speed must be finite and {allowed} cells per second"),
        ));
    }
    Ok(())
}

fn validate_gradient(colors: &[Rgba], min: usize, layer_id: &str) -> Result<(), EngineError> {
    if colors.len() < min || colors.len() > MAX_GRADIENT_COLORS {
        return Err(EngineError::new(
            "invalid_effect_parameter",
            format!(
                "layer '{layer_id}' gradient must contain between {min} and {MAX_GRADIENT_COLORS} colors"
            ),
        ));
    }
    for color in colors {
        validate_color(*color, layer_id)?;
    }
    Ok(())
}

fn sample_effect(
    effect: &Effect,
    led: &LedPosition,
    time_ms: u64,
    events: &VecDeque<KeyEvent>,
    key_positions: &HashMap<KeyId, (u16, u16)>,
) -> Option<Rgba> {
    match effect {
        Effect::Static { color } => Some(*color),
        Effect::Breathing { color, period_ms } => {
            let phase = (time_ms % period_ms) as f64 / *period_ms as f64;
            let intensity = (0.5 - 0.5 * (std::f64::consts::TAU * phase).cos()) as f32;
            Some(with_intensity(*color, intensity))
        }
        Effect::Spectrum {
            period_ms,
            wavelength_cells,
        } => {
            let temporal = (time_ms % period_ms) as f64 / *period_ms as f64;
            let phase = temporal + f64::from(led.x) / f64::from(*wavelength_cells);
            Some(hsv_to_rgba((phase.rem_euclid(1.0) * 360.0) as f32))
        }
        Effect::Wave {
            colors,
            speed_cells_per_second,
            wavelength_cells,
            direction,
        } => {
            let coordinate = match direction {
                WaveDirection::Horizontal => f64::from(led.x),
                WaveDirection::Vertical => f64::from(led.y),
                WaveDirection::Diagonal => (f64::from(led.x) + f64::from(led.y)) / 2.0_f64.sqrt(),
            };
            let travel = f64::from(*speed_cells_per_second) * time_ms as f64 / 1000.0;
            let phase = (coordinate - travel) / f64::from(*wavelength_cells);
            Some(sample_gradient(colors, phase.rem_euclid(1.0) as f32))
        }
        Effect::Reactive { color, fade_ms } => {
            let target_key = led.key_id.as_ref()?;
            for event in events.iter().rev() {
                if &event.key_id != target_key || event.time_ms > time_ms {
                    continue;
                }
                let age_ms = time_ms - event.time_ms;
                if age_ms >= *fade_ms {
                    return None;
                }
                let intensity = 1.0 - age_ms as f32 / *fade_ms as f32;
                return Some(with_intensity(*color, intensity));
            }
            None
        }
        Effect::Ripple {
            colors,
            duration_ms,
            speed_cells_per_second,
            width_cells,
        } => {
            let mut best: Option<(f32, Rgba)> = None;
            for event in events {
                if event.time_ms > time_ms {
                    continue;
                }
                let age_ms = time_ms - event.time_ms;
                if age_ms >= *duration_ms {
                    continue;
                }
                let Some((origin_x, origin_y)) = key_positions.get(&event.key_id) else {
                    continue;
                };
                let dx = f64::from(led.x) - f64::from(*origin_x);
                let dy = f64::from(led.y) - f64::from(*origin_y);
                let distance = dx.hypot(dy);
                let radius = f64::from(*speed_cells_per_second) * age_ms as f64 / 1000.0;
                let half_width = f64::from(*width_cells) / 2.0;
                let strength =
                    (1.0 - (distance - radius).abs() / half_width).clamp(0.0, 1.0) as f32;
                if strength == 0.0 || best.as_ref().is_some_and(|(value, _)| strength <= *value) {
                    continue;
                }
                let max_radius = f64::from(*speed_cells_per_second) * *duration_ms as f64 / 1000.0;
                let progress = if max_radius == 0.0 {
                    0.0
                } else {
                    radius / max_radius
                };
                best = Some((
                    strength,
                    with_intensity(sample_gradient(colors, progress as f32), strength),
                ));
            }
            best.map(|(_, color)| color)
        }
    }
}

fn with_intensity(mut color: Rgba, intensity: f32) -> Rgba {
    color.a *= intensity.clamp(0.0, 1.0);
    color
}

fn sample_gradient(colors: &[Rgba], position: f32) -> Rgba {
    if colors.len() == 1 {
        return colors[0];
    }
    let scaled = position.clamp(0.0, 1.0) * (colors.len() - 1) as f32;
    let index = (scaled.floor() as usize).min(colors.len() - 2);
    let fraction = scaled - index as f32;
    let start = colors[index];
    let end = colors[index + 1];
    Rgba::new(
        lerp(start.r, end.r, fraction),
        lerp(start.g, end.g, fraction),
        lerp(start.b, end.b, fraction),
        lerp(start.a, end.a, fraction),
    )
}

fn hsv_to_rgba(hue: f32) -> Rgba {
    let sector = hue.rem_euclid(360.0) / 60.0;
    let chroma = 1.0;
    let x = chroma * (1.0 - (sector.rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match sector as u8 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    Rgba::rgb(r, g, b)
}

fn lerp(start: f32, end: f32, amount: f32) -> f32 {
    start + (end - start) * amount
}

fn apply_blend(destination: &mut [f32; 3], sample: Rgba, coverage: f32, mode: BlendMode) {
    match mode {
        BlendMode::Over => {
            destination[0] = sample.r * coverage + destination[0] * (1.0 - coverage);
            destination[1] = sample.g * coverage + destination[1] * (1.0 - coverage);
            destination[2] = sample.b * coverage + destination[2] * (1.0 - coverage);
        }
        BlendMode::Add => {
            destination[0] = (destination[0] + sample.r * coverage).min(1.0);
            destination[1] = (destination[1] + sample.g * coverage).min(1.0);
            destination[2] = (destination[2] + sample.b * coverage).min(1.0);
        }
        BlendMode::Multiply => {
            destination[0] *= 1.0 - coverage + sample.r * coverage;
            destination[1] *= 1.0 - coverage + sample.g * coverage;
            destination[2] *= 1.0 - coverage + sample.b * coverage;
        }
    }
}

fn to_channel(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

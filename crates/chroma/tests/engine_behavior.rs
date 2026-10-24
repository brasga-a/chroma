use chroma::{
    BlendMode, Command, Effect, Engine, KeyId, KeyPosition, Layer, Layout, LedPosition, MAX_EVENTS,
    Request, Rgba, Scene, WaveDirection,
};
use std::collections::BTreeSet;

fn layout_with_keys(width: u16) -> Layout {
    let keys: Vec<_> = (0..width)
        .map(|x| KeyPosition::new(format!("key-{x}"), x, 0))
        .collect();
    let leds = keys
        .iter()
        .map(|key| {
            LedPosition::new(
                format!("led-{}", key.id.0),
                Some(key.id.clone()),
                key.x,
                key.y,
            )
        })
        .collect();
    Layout::new("test-layout", width, 1, keys, leds)
}

fn static_layer(id: &str, color: Rgba) -> Layer {
    Layer::new(id, Effect::Static { color })
}

fn engine_with_scene(layout: Layout, scene: Scene) -> Engine {
    Engine::new(layout, scene).expect("fixture is valid")
}

#[test]
fn ordered_layers_apply_opacity_masks_and_disabled_layers_without_erasing_lower_pixels() {
    let layout = layout_with_keys(3);
    let base = static_layer("base", Rgba::rgb(0.0, 0.0, 1.0));
    let mut overlay = Layer::new(
        "overlay",
        Effect::Static {
            color: Rgba::new(1.0, 0.0, 0.0, 0.5),
        },
    );
    overlay.opacity = 0.5;
    overlay.mask = Some([KeyId::from("key-1")].into());

    let mut disabled = static_layer("disabled", Rgba::rgb(0.0, 1.0, 0.0));
    disabled.enabled = false;
    let mut empty_mask = static_layer("empty-mask", Rgba::rgb(1.0, 1.0, 1.0));
    empty_mask.mask = Some(BTreeSet::new());

    let engine = engine_with_scene(
        layout,
        Scene::new("composition", vec![base, overlay, disabled, empty_mask]),
    );
    let frame = engine.render_at(20);

    assert_eq!(
        frame.cell_at(0, 0).unwrap().rgb,
        chroma::Rgb::new(0, 0, 255)
    );
    assert_eq!(
        frame.cell_at(1, 0).unwrap().rgb,
        chroma::Rgb::new(64, 0, 191)
    );
    assert_eq!(
        frame.cell_at(2, 0).unwrap().rgb,
        chroma::Rgb::new(0, 0, 255)
    );
}

#[test]
fn add_and_multiply_use_alpha_weighted_coverage_and_clamp_rgb() {
    let layout = layout_with_keys(1);
    let base = static_layer("base", Rgba::rgb(0.8, 0.6, 0.4));
    let mut add = Layer::new(
        "add",
        Effect::Static {
            color: Rgba::new(0.5, 1.0, 1.0, 0.5),
        },
    );
    add.blend = BlendMode::Add;
    add.opacity = 0.5;
    let mut multiply = Layer::new(
        "multiply",
        Effect::Static {
            color: Rgba::new(0.5, 1.0, 0.0, 0.5),
        },
    );
    multiply.blend = BlendMode::Multiply;
    multiply.opacity = 0.5;

    let engine = engine_with_scene(layout, Scene::new("blend-modes", vec![base, add, multiply]));
    // Add first produces (.925, .85, .65); multiply coverage is .25.
    let rgb = engine.render_at(0).cell_at(0, 0).unwrap().rgb;
    assert_eq!(rgb, chroma::Rgb::new(206, 217, 124));

    let mut transparent = Layer::new(
        "transparent-add",
        Effect::Static {
            color: Rgba::new(1.0, 1.0, 1.0, 0.0),
        },
    );
    transparent.blend = BlendMode::Add;
    let engine = engine_with_scene(
        layout_with_keys(1),
        Scene::new(
            "alpha-zero",
            vec![static_layer("base", Rgba::rgb(0.2, 0.3, 0.4)), transparent],
        ),
    );
    assert_eq!(
        engine.render_at(0).cell_at(0, 0).unwrap().rgb,
        chroma::Rgb::new(51, 77, 102)
    );
}

#[test]
fn frame_preserves_matrix_holes_and_led_identity() {
    let layout = Layout::new(
        "sparse",
        3,
        1,
        vec![
            KeyPosition::new("left", 0, 0),
            KeyPosition::new("right", 2, 0),
        ],
        vec![
            LedPosition::new("stable-left-led", Some(KeyId::from("left")), 0, 0),
            LedPosition::new("stable-right-led", Some(KeyId::from("right")), 2, 0),
        ],
    );
    let engine = engine_with_scene(
        layout,
        Scene::new(
            "sparse-scene",
            vec![static_layer("all", Rgba::rgb(1.0, 0.0, 0.0))],
        ),
    );
    let frame = engine.render_at(123);

    assert_eq!(frame.cells.len(), 3);
    assert_eq!(frame.cell_at(1, 0), None);
    assert_eq!(frame.cell_at(0, 0).unwrap().led_id.0, "stable-left-led");
    assert_eq!(
        frame.cell_at(2, 0).unwrap().key_id.as_ref().unwrap().0,
        "right"
    );
    assert_eq!(
        frame.cell_at(2, 0).unwrap().rgb,
        chroma::Rgb::new(255, 0, 0)
    );
    assert_eq!(frame.layout_id, "sparse");
}

#[test]
fn breathing_spectrum_and_wave_change_deterministically_with_absolute_time() {
    let layout = layout_with_keys(4);
    let breathing = Layer::new(
        "breathe",
        Effect::Breathing {
            color: Rgba::rgb(0.0, 1.0, 0.0),
            period_ms: 1000,
        },
    );
    let engine = engine_with_scene(layout, Scene::new("breathing", vec![breathing]));
    assert_eq!(
        engine.render_at(0).cell_at(0, 0).unwrap().rgb,
        chroma::Rgb::BLACK
    );
    assert_eq!(
        engine.render_at(500).cell_at(0, 0).unwrap().rgb,
        chroma::Rgb::new(0, 255, 0)
    );
    assert_eq!(
        engine.render_at(1000).cell_at(0, 0).unwrap().rgb,
        engine.render_at(0).cell_at(0, 0).unwrap().rgb
    );

    let spectrum = Layer::new(
        "spectrum",
        Effect::Spectrum {
            period_ms: 1000,
            wavelength_cells: 4.0,
        },
    );
    let engine = engine_with_scene(layout_with_keys(1), Scene::new("spectrum", vec![spectrum]));
    assert_eq!(
        engine.render_at(250).cell_at(0, 0).unwrap().rgb,
        chroma::Rgb::new(128, 255, 0)
    );

    let wave = Layer::new(
        "wave",
        Effect::Wave {
            colors: vec![Rgba::rgb(1.0, 0.0, 0.0), Rgba::rgb(0.0, 0.0, 1.0)],
            speed_cells_per_second: 1.0,
            wavelength_cells: 2.0,
            direction: WaveDirection::Horizontal,
        },
    );
    let engine = engine_with_scene(layout_with_keys(2), Scene::new("wave", vec![wave]));
    let at_zero = engine.render_at(0);
    assert_eq!(
        at_zero.cell_at(0, 0).unwrap().rgb,
        chroma::Rgb::new(255, 0, 0)
    );
    assert_eq!(
        at_zero.cell_at(1, 0).unwrap().rgb,
        chroma::Rgb::new(128, 0, 128)
    );
    let after_one_second = engine.render_at(1000);
    assert_eq!(
        after_one_second.cell_at(0, 0).unwrap().rgb,
        chroma::Rgb::new(128, 0, 128)
    );
    assert_eq!(
        after_one_second.cell_at(1, 0).unwrap().rgb,
        chroma::Rgb::new(255, 0, 0)
    );
}

#[test]
fn reactive_and_ripple_events_have_defined_fades_and_expiration() {
    let reactive = Layer::new(
        "reactive",
        Effect::Reactive {
            color: Rgba::rgb(1.0, 0.0, 0.0),
            fade_ms: 100,
        },
    );
    let mut engine = engine_with_scene(
        layout_with_keys(3),
        Scene::new(
            "reactive-scene",
            vec![static_layer("under", Rgba::rgb(0.0, 0.0, 1.0)), reactive],
        ),
    );
    engine.key_press(KeyId::from("key-0"), 10).unwrap();
    engine.key_press(KeyId::from("key-0"), 20).unwrap();
    assert_eq!(
        engine.render_at(20).cell_at(0, 0).unwrap().rgb,
        chroma::Rgb::new(255, 0, 0)
    );
    assert_eq!(
        engine.render_at(60).cell_at(0, 0).unwrap().rgb,
        chroma::Rgb::new(153, 0, 102)
    );
    assert_eq!(
        engine.render_at(120).cell_at(0, 0).unwrap().rgb,
        chroma::Rgb::new(0, 0, 255)
    );
    assert_eq!(
        engine.render_at(60).cell_at(1, 0).unwrap().rgb,
        chroma::Rgb::new(0, 0, 255)
    );

    let ripple = Layer::new(
        "ripple",
        Effect::Ripple {
            colors: vec![Rgba::rgb(1.0, 0.0, 0.0), Rgba::rgb(0.0, 0.0, 1.0)],
            duration_ms: 2000,
            speed_cells_per_second: 1.0,
            width_cells: 1.0,
        },
    );
    let mut engine = engine_with_scene(
        layout_with_keys(3),
        Scene::new(
            "ripple-scene",
            vec![static_layer("under", Rgba::rgb(0.0, 0.4, 0.0)), ripple],
        ),
    );
    engine.key_press(KeyId::from("key-0"), 0).unwrap();
    assert_eq!(
        engine.render_at(0).cell_at(0, 0).unwrap().rgb,
        chroma::Rgb::new(255, 0, 0)
    );
    assert_eq!(
        engine.render_at(1000).cell_at(1, 0).unwrap().rgb,
        chroma::Rgb::new(128, 0, 128)
    );
    assert_eq!(
        engine.render_at(1000).cell_at(2, 0).unwrap().rgb,
        chroma::Rgb::new(0, 102, 0)
    );
    assert_eq!(
        engine.render_at(2000).cell_at(0, 0).unwrap().rgb,
        chroma::Rgb::new(0, 102, 0)
    );

    let ripple = Layer::new(
        "overlapping-ripples",
        Effect::Ripple {
            colors: vec![Rgba::rgb(1.0, 0.0, 0.0), Rgba::rgb(0.0, 0.0, 1.0)],
            duration_ms: 2000,
            speed_cells_per_second: 1.0,
            width_cells: 2.0,
        },
    );
    let mut engine = engine_with_scene(
        layout_with_keys(3),
        Scene::new(
            "overlapping-scene",
            vec![static_layer("under", Rgba::rgb(0.0, 0.4, 0.0)), ripple],
        ),
    );
    engine.key_press(KeyId::from("key-0"), 0).unwrap();
    engine.key_press(KeyId::from("key-2"), 500).unwrap();
    let frame = engine.render_at(1500);
    // At key-1 both rings overlap; the second ring has full coverage and wins.
    assert_eq!(
        frame.cell_at(1, 0).unwrap().rgb,
        chroma::Rgb::new(128, 0, 128)
    );
    // At key-2 only the first ring is visible at half coverage.
    assert_eq!(
        frame.cell_at(2, 0).unwrap().rgb,
        chroma::Rgb::new(32, 51, 96)
    );
    let expired = engine.render_at(2500);
    for x in 0..expired.width {
        assert_eq!(
            expired.cell_at(x, 0).unwrap().rgb,
            chroma::Rgb::new(0, 102, 0)
        );
    }
}

#[test]
fn event_history_is_bounded_render_is_idempotent_and_u64_timestamps_do_not_overflow() {
    let layer = Layer::new(
        "reactive",
        Effect::Reactive {
            color: Rgba::rgb(1.0, 0.0, 0.0),
            fade_ms: 100,
        },
    );
    let mut engine =
        Engine::with_event_capacity(layout_with_keys(2), Scene::new("bounded", vec![layer]), 2)
            .unwrap();
    engine
        .key_press(KeyId::from("key-0"), u64::MAX - 2)
        .unwrap();
    engine
        .key_press(KeyId::from("key-1"), u64::MAX - 1)
        .unwrap();
    let first = engine.render_at(u64::MAX);
    assert_eq!(first, engine.render_at(u64::MAX));
    assert_eq!(engine.key_press(KeyId::from("key-0"), u64::MAX).unwrap(), 2);
    assert_eq!(engine.retained_event_count(), 2);
    let before_newest = engine.render_at(u64::MAX - 1);
    assert_eq!(before_newest.cell_at(0, 0).unwrap().rgb, chroma::Rgb::BLACK);
    assert_eq!(
        before_newest.cell_at(1, 0).unwrap().rgb,
        chroma::Rgb::new(255, 0, 0)
    );

    let before = engine.retained_event_count();
    assert_eq!(
        engine
            .key_press(KeyId::from("key-0"), u64::MAX - 10)
            .unwrap_err()
            .code,
        "out_of_order_event"
    );
    assert_eq!(engine.retained_event_count(), before);
    assert!(Engine::with_event_capacity(layout_with_keys(1), Scene::new("x", vec![]), 0).is_err());
    assert!(
        Engine::with_event_capacity(layout_with_keys(1), Scene::new("x", vec![]), MAX_EVENTS + 1)
            .is_err()
    );
}

#[test]
fn failed_scene_and_layout_replacements_leave_active_state_unchanged() {
    let mut base = static_layer("red", Rgba::rgb(1.0, 0.0, 0.0));
    base.mask = Some([KeyId::from("key-0")].into());
    let mut engine = engine_with_scene(layout_with_keys(1), Scene::new("valid", vec![base]));
    engine.key_press(KeyId::from("key-0"), 0).unwrap();
    let mut invalid_scene = Scene::new(
        "invalid",
        vec![static_layer("green", Rgba::rgb(0.0, 1.0, 0.0))],
    );
    invalid_scene.layers[0].opacity = f32::NAN;
    assert_eq!(
        engine.set_scene(invalid_scene).unwrap_err().code,
        "invalid_opacity"
    );
    assert_eq!(engine.scene().id, "valid");
    assert_eq!(
        engine.render_at(0).cell_at(0, 0).unwrap().rgb,
        chroma::Rgb::new(255, 0, 0)
    );

    let old_layout_id = engine.layout().id.clone();
    let candidate = Layout::new(
        "missing-key-layout",
        1,
        1,
        vec![],
        vec![LedPosition::new("unmapped", None, 0, 0)],
    );
    assert_eq!(
        engine.set_layout(candidate).unwrap_err().code,
        "unknown_key"
    );
    assert_eq!(engine.layout().id, old_layout_id);
    assert_eq!(engine.retained_event_count(), 1);
    assert_eq!(
        engine.render_at(0).cell_at(0, 0).unwrap().rgb,
        chroma::Rgb::new(255, 0, 0)
    );
}

#[test]
fn json_roundtrip_and_versioned_errors_preserve_the_scene() {
    let mut engine = engine_with_scene(
        layout_with_keys(1),
        Scene::new("kept", vec![static_layer("blue", Rgba::rgb(0.0, 0.0, 1.0))]),
    );
    let request = Request::new(Command::RenderAt { time_ms: 42 });
    let encoded = serde_json::to_vec(&request).unwrap();
    let decoded: Request = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded, request);
    let response = engine.handle_request(decoded);
    assert!(response.ok);
    let response_bytes = serde_json::to_vec(&response).unwrap();
    let response_roundtrip: chroma::Response = serde_json::from_slice(&response_bytes).unwrap();
    assert_eq!(response_roundtrip, response);
    let response_json = serde_json::to_value(response).unwrap();
    assert_eq!(response_json["version"], 1);
    assert_eq!(response_json["result"]["frame"]["time_ms"], 42);

    let bad_scene = serde_json::json!({
        "version": 1,
        "command": {
            "type": "set_scene",
            "scene": {
                "id": "bad",
                "layers": [{
                    "id": "bad-opacity",
                    "enabled": true,
                    "opacity": 2.0,
                    "mask": null,
                    "blend": "over",
                    "effect": {"type": "static", "color": {"r": 1.0, "g": 0.0, "b": 0.0, "a": 1.0}}
                }]
            }
        }
    });
    let decoded: Request = serde_json::from_value(bad_scene).unwrap();
    let response = engine.handle_request(decoded);
    assert!(!response.ok);
    assert_eq!(response.error.as_ref().unwrap().code, "invalid_opacity");
    let response_bytes = serde_json::to_vec(&response).unwrap();
    let response_roundtrip: chroma::Response = serde_json::from_slice(&response_bytes).unwrap();
    assert_eq!(response_roundtrip, response);
    assert_eq!(engine.scene().id, "kept");
    assert_eq!(
        engine.render_at(42).cell_at(0, 0).unwrap().rgb,
        chroma::Rgb::new(0, 0, 255)
    );

    let response = engine.handle_request(Request {
        version: 99,
        command: Command::RenderAt { time_ms: 0 },
    });
    assert!(!response.ok);
    assert_eq!(response.error.unwrap().code, "unsupported_version");
    assert_eq!(engine.scene().id, "kept");
}

#[test]
fn scenes_are_reusable_across_independent_engines() {
    let scene = Scene::new(
        "shared",
        vec![Layer::new(
            "wave",
            Effect::Wave {
                colors: vec![Rgba::rgb(0.0, 0.0, 0.0), Rgba::rgb(1.0, 0.5, 0.0)],
                speed_cells_per_second: 0.5,
                wavelength_cells: 4.0,
                direction: WaveDirection::Horizontal,
            },
        )],
    );
    let first = engine_with_scene(layout_with_keys(4), scene.clone());
    let second = engine_with_scene(layout_with_keys(4), scene.clone());
    assert_eq!(first.render_at(250), second.render_at(250));
    assert_eq!(scene.id, "shared");
    assert_eq!(scene.layers.len(), 1);
}

#[test]
fn successful_layout_changes_clear_events_and_configuration_swaps_atomically() {
    let reactive = Layer::new(
        "reactive",
        Effect::Reactive {
            color: Rgba::rgb(1.0, 0.0, 0.0),
            fade_ms: 1000,
        },
    );
    let mut engine = engine_with_scene(
        layout_with_keys(1),
        Scene::new("old", vec![reactive.clone()]),
    );
    engine.key_press(KeyId::from("key-0"), 100).unwrap();
    assert_eq!(engine.retained_event_count(), 1);

    let bad_layout = Layout::new(
        "bad-layout",
        1,
        1,
        vec![],
        vec![LedPosition::new("unmapped", None, 0, 0)],
    );
    let mut invalid_reactive = reactive.clone();
    invalid_reactive.mask = Some([KeyId::from("missing-key")].into());
    let bad_scene = Scene::new("bad-scene", vec![invalid_reactive]);
    assert!(engine.set_configuration(bad_layout, bad_scene).is_err());
    assert_eq!(engine.layout().id, "test-layout");
    assert_eq!(engine.scene().id, "old");
    assert_eq!(engine.retained_event_count(), 1);

    let new_layout = layout_with_keys(2);
    let new_scene = Scene::new("new", vec![reactive]);
    let command = Command::SetConfiguration {
        layout: new_layout,
        scene: new_scene,
    };
    let encoded = serde_json::to_vec(&Request::new(command.clone())).unwrap();
    let decoded: Request = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded.command, command);
    let response = engine.handle_request(decoded);
    assert!(response.ok);
    assert_eq!(engine.layout().id, "test-layout");
    assert_eq!(engine.scene().id, "new");
    assert_eq!(engine.retained_event_count(), 0);
    // The new layout starts a fresh event timeline after the successful commit.
    assert!(engine.key_press(KeyId::from("key-0"), 1).is_ok());

    // The ordinary layout setter also clears events after a successful commit.
    engine.key_press(KeyId::from("key-1"), 2).unwrap();
    assert_eq!(engine.retained_event_count(), 2);
    engine.set_layout(layout_with_keys(1)).unwrap();
    assert_eq!(engine.retained_event_count(), 0);
}

#[test]
fn layout_and_effect_parameter_limits_are_rejected() {
    let invalid_dimensions = Layout::new("zero", 0, 1, vec![], vec![]);
    assert_eq!(
        Engine::new(invalid_dimensions, Scene::new("empty", vec![]))
            .unwrap_err()
            .code,
        "invalid_dimensions"
    );

    let duplicate = Layout::new(
        "duplicate",
        2,
        1,
        vec![
            KeyPosition::new("same", 0, 0),
            KeyPosition::new("same", 1, 0),
        ],
        vec![],
    );
    assert_eq!(
        Engine::new(duplicate, Scene::new("empty", vec![]))
            .unwrap_err()
            .code,
        "duplicate_id"
    );

    let bad_coordinate = Layout::new(
        "coordinate",
        1,
        1,
        vec![KeyPosition::new("key", 1, 0)],
        vec![],
    );
    assert_eq!(
        Engine::new(bad_coordinate, Scene::new("empty", vec![]))
            .unwrap_err()
            .code,
        "invalid_coordinate"
    );

    let zero_period = Layer::new(
        "zero-period",
        Effect::Breathing {
            color: Rgba::rgb(1.0, 1.0, 1.0),
            period_ms: 0,
        },
    );
    assert_eq!(
        Engine::new(
            layout_with_keys(1),
            Scene::new("invalid", vec![zero_period])
        )
        .unwrap_err()
        .code,
        "invalid_effect_parameter"
    );

    let nan_wave = Layer::new(
        "nan-speed",
        Effect::Wave {
            colors: vec![Rgba::rgb(0.0, 0.0, 0.0), Rgba::rgb(1.0, 1.0, 1.0)],
            speed_cells_per_second: f32::INFINITY,
            wavelength_cells: 1.0,
            direction: WaveDirection::Horizontal,
        },
    );
    assert_eq!(
        Engine::new(layout_with_keys(1), Scene::new("invalid", vec![nan_wave]))
            .unwrap_err()
            .code,
        "invalid_effect_parameter"
    );

    let tiny_ripple = Layer::new(
        "tiny-ripple",
        Effect::Ripple {
            colors: vec![Rgba::rgb(1.0, 0.0, 0.0)],
            duration_ms: 100,
            speed_cells_per_second: 1.0,
            width_cells: f32::MIN_POSITIVE,
        },
    );
    assert_eq!(
        Engine::new(
            layout_with_keys(1),
            Scene::new("invalid", vec![tiny_ripple])
        )
        .unwrap_err()
        .code,
        "invalid_effect_parameter"
    );

    let too_many_layers = Scene::new(
        "full",
        (0..=chroma::MAX_LAYERS)
            .map(|index| static_layer(&format!("layer-{index}"), Rgba::rgb(0.0, 0.0, 0.0)))
            .collect(),
    );
    assert_eq!(
        Engine::new(layout_with_keys(1), too_many_layers)
            .unwrap_err()
            .code,
        "too_many_layers"
    );
}

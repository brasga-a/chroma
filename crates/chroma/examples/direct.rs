use chroma::{
    BlendMode, Effect, Engine, KeyId, KeyPosition, Layer, Layout, LedPosition, Rgba, Scene,
    WaveDirection,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let layout = Layout::new(
        "example-layout",
        3,
        1,
        vec![
            KeyPosition::new("key-a", 0, 0),
            KeyPosition::new("key-b", 1, 0),
            KeyPosition::new("key-c", 2, 0),
        ],
        vec![
            LedPosition::new("led-a", Some(KeyId::from("key-a")), 0, 0),
            LedPosition::new("led-b", Some(KeyId::from("key-b")), 1, 0),
            LedPosition::new("led-c", Some(KeyId::from("key-c")), 2, 0),
        ],
    );

    // Layer order is bottom to top. The base provides a steady color while
    // the wave moves across the top two keys and blends over it.
    let mut wave = Layer::new(
        "wave",
        Effect::Wave {
            colors: vec![Rgba::rgb(0.0, 0.1, 1.0), Rgba::rgb(1.0, 0.2, 0.0)],
            speed_cells_per_second: 0.8,
            wavelength_cells: 3.0,
            direction: WaveDirection::Horizontal,
        },
    );
    wave.opacity = 0.75;
    wave.blend = BlendMode::Add;
    wave.mask = Some([KeyId::from("key-b"), KeyId::from("key-c")].into());

    let scene = Scene::new(
        "desk-scene",
        vec![
            Layer::new(
                "base",
                Effect::Static {
                    color: Rgba::rgb(0.0, 0.0, 0.12),
                },
            ),
            wave,
            Layer::new(
                "key-press",
                Effect::Reactive {
                    color: Rgba::rgb(1.0, 1.0, 1.0),
                    fade_ms: 400,
                },
            ),
        ],
    );

    let mut engine = Engine::new(layout, scene)?;
    engine.key_press(KeyId::from("key-b"), 800)?;
    let frame = engine.render_at(950);

    println!("layout={} time={}ms", frame.layout_id, frame.time_ms);
    for x in 0..frame.width {
        let cell = frame
            .cell_at(x, 0)
            .expect("each example coordinate has an LED");
        println!(
            "{}: rgb({},{},{})",
            cell.led_id.0, cell.rgb.r, cell.rgb.g, cell.rgb.b
        );
    }

    Ok(())
}

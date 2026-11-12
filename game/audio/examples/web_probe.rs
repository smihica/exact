//! Size probe: cargo build -p exact-game-audio --example web_probe --profile web --target wasm32-unknown-unknown
#[cfg(target_arch = "wasm32")]
mod probe {
    use exact_game::{
        audio::{self, Synth},
        Clock, Game, Input, Sim, World,
    };
    struct Tone;
    impl Game for Tone {
        const ID: &'static str = "finite-pitch-tail";
        type Args = ();
        fn setup(w: &mut World, _: &()) {
            w.sounds([("tone", Synth::sine(100.).sustain(1.).seconds(1.))]);
            w.play("tone").start();
        }
        fn tick(w: &mut World, _: &Input, _: &()) {
            audio::step(w);
        }
    }
    use exact_game_audio::{Output, Pcm, Player, Transport, WebOutput};
    use wasm_bindgen::prelude::*;
    #[wasm_bindgen]
    pub struct Probe {
        player: Player<WebOutput>,
        world: World,
        transport: Transport,
        sim: Sim<Tone>,
    }
    #[wasm_bindgen]
    impl Probe {
        #[wasm_bindgen(constructor)]
        pub fn new() -> Result<Probe, JsValue> {
            let mut world = World::new(60, 0);
            world.sounds([("chime", Synth::sine(880.0))]);
            world.play("chime").ui().start();
            Ok(Self {
                player: Player::new(WebOutput::new()?, 48000),
                world,
                transport: Transport::default(),
                sim: Sim::new(()).unwrap(),
            })
        }
        pub fn unlock(&mut self) {
            self.player.output.unlock();
        }
        pub fn ready(&self) -> bool {
            self.player.output.ready()
        }
        pub fn frame(&mut self) {
            self.player.sync(&self.world, None, self.transport);
        }
        pub fn full_mix(&mut self) {
            let w = self.sim.world_mut();
            w.sounds([("loop", Synth::sine(200.).seconds(1.).looped())]);
            audio::pitch(w, 0, 16.);
            for _ in 0..31 {
                w.play("loop").start();
            }
        }
        pub fn add_voice(&mut self) {
            self.sim.world_mut().play("tone").start();
        }
        fn advance(&mut self, tick: u32, pitch: f32) {
            self.sim.advance(tick as f64 * 1000. / 60., Clock::Seekable);
            if pitch > 0. {
                audio::pitch(self.sim.world_mut(), 0, pitch);
            }
        }
        pub fn tick(&mut self, tick: u32, pitch: f32) {
            self.advance(tick, pitch);
            self.player.sync(self.sim.world(), None, self.transport);
        }
        pub fn cached_sounds(&self) -> usize {
            self.player.cached_sounds()
        }
        pub fn start(&mut self, samples: Vec<f32>, rate: u32) -> bool {
            let pcm = Pcm::F32(samples.into());
            self.player.output.start(1, &pcm, rate, false, 0, 1.0)
        }
        pub fn pitch(&mut self, pitch: f32) {
            self.player.output.set_pitch(1, pitch);
        }
        pub fn set(&mut self, left: f32, right: f32) {
            self.player.output.set(1, left, right);
        }
    }
}

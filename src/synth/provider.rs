use toybox::prelude::*;

use std::sync::mpsc::Receiver;

use super::adsr::Adsr;


#[derive(Debug)]
pub enum SynthMessage {
	NoteOn {
		note: u8,
		velocity: u8,
	},

	NoteOff(u8),
}



pub struct SynthProvider {
	msg_rx: Receiver<SynthMessage>,
	voice_bank: VoiceBank,

	sample_rate: u32,
	channels: usize,
}

impl SynthProvider {
	pub fn new(msg_rx: Receiver<SynthMessage>) -> Self {
		SynthProvider {
			msg_rx,
			voice_bank: VoiceBank::new(),

			sample_rate: 44100,
			channels: 2,
		}
	}

	fn process_messages(&mut self) {
		use SynthMessage::*;

		for msg in self.msg_rx.try_iter() {
			match msg {
				NoteOff(note) => self.voice_bank.note_off(note),
				NoteOn{note, velocity} => self.voice_bank.note_on(note, velocity),
			}
		}
	}
}

impl audio::Provider for SynthProvider {
	fn on_configuration_changed(&mut self, audio::Configuration{sample_rate, channels}: audio::Configuration) {
		self.sample_rate = sample_rate;
		self.channels = channels;
	}

	fn fill_buffer(&mut self, buffer: &mut [f32]) {
		self.process_messages();

		assert!(self.channels == 2);

		// let buffer_size = buffer.len();
		// let sample_buffer = match self.channels {
		// 	1 => &mut buffer[..],
		// 	2 => &mut buffer[..buffer_size/2],

		// 	n => panic!("Unsupported number of channels: {n}"),
		// };

		// Evaluate samples
		buffer.fill(0.0);

		let sample_dt = (self.sample_rate as f32).recip();

		let (buffer_stereo, _) = buffer.as_chunks_mut();

		for voice in self.voice_bank.voices.iter_mut() {
			voice.update_and_fill(buffer_stereo, sample_dt);
		}

		self.voice_bank.clean_up();

		// // Mono -> Stereo
		// for idx in (0..buffer_size/2).rev() {
		// 	let sample = buffer[idx];
		// 	buffer[idx*2+1] = sample;
		// 	buffer[idx*2] = sample;
		// }
	}
}




struct VoiceBank {
	voices: Vec<Voice>,

	pan_seed: f32,
}

impl VoiceBank {
	fn new() -> Self {
		VoiceBank {
			voices: Vec::with_capacity(32),
			pan_seed: 0.0,
		}
	}

	fn note_off(&mut self, note: u8) {
		for voice in self.voices.iter_mut() {
			if voice.note == note {
				voice.release();
				break
			}
		}
	}

	fn note_on(&mut self, note: u8, velocity: u8) {
		let gain = midi_velocity_to_gain(velocity);

		if let Some(voice) = self.voices.iter_mut().find(|v| v.note == note) {
			voice.restart(gain);
		} else {
			let pan = (self.pan_seed - 0.5) * 1.2;

			self.pan_seed = (self.pan_seed + 2503.0 / 443.0).fract();

			self.voices.push(Voice::new(note, gain, pan));
		}
	}

	fn clean_up(&mut self) {
		self.voices.retain(|v| !v.is_silent());
	}
}



struct Voice {
	phase: f32,
	adsr: Adsr,
	active: bool,
	silence_timer: u8,

	gain: f32,
	gain_filter: BasicLP,

	pan: f32,

	note: u8,
}

impl Voice {
	fn new(note: u8, gain: f32, pan: f32) -> Voice {
		Voice {
			note,
			adsr: Adsr::new(0.03, 0.2, 0.5, 4.0),

			active: true,
			silence_timer: 0,

			gain,
			gain_filter: BasicLP::new(300.0),

			pan: (pan * 0.5 + 0.5).clamp(0.0, 1.0),

			phase: 0.0,
		}
	}

	fn restart(&mut self, gain: f32) {
		self.gain = gain;
		self.active = true;
		self.silence_timer = 0;
	}

	fn release(&mut self) {
		self.active = false;
	}

	fn is_silent(&self) -> bool {
		self.silence_timer > 4
	}

	fn update_and_fill(&mut self, out: &mut [[f32; 2]], sample_dt: f32) {
		let freq = midi_note_to_freq(self.note);
		let inc = TAU * freq * sample_dt;

		self.gain_filter.set_sample_dt(sample_dt);

		let l_gain = (self.pan).sqrt();
		let r_gain = (1.0 - self.pan).sqrt();

		for [l_sample, r_sample] in out {
			let env = self.adsr.advance(sample_dt, self.active);
			let env = env * env;

			let osc = self.phase.sin() * 3.0
				+ (self.phase * 3.0).sin() * 2.0
				+ (self.phase * 5.0).sin() * 1.0
				+ (self.phase * 7.0).sin() * 0.0;
			let osc = osc / 6.0;

			let sample = osc * env * self.gain_filter.evaluate(self.gain);

			*l_sample += sample * l_gain;
			*r_sample += sample * r_gain;

			self.phase += inc;
		}

		self.phase = self.phase % TAU;

		if self.adsr.is_silent() {
			self.silence_timer = self.silence_timer.saturating_add(1);
		}
	}
}




fn midi_note_to_freq(note: u8) -> f32 {
    ((note as f32 - 69.0) / 12.0).exp2() * 440.0
}

fn midi_velocity_to_gain(velocity: u8) -> f32 {
	(velocity.min(127) as f32 / 127.0).powi(2) * 0.5
}


struct BasicLP {
	freq: f32,

	alpha: f32,
	prev_value: f32,
}

impl BasicLP {
	fn new(freq: f32) -> BasicLP {
		BasicLP {
			freq,

			alpha: 0.0,
			prev_value: 0.0,
		}
	}

	fn set_sample_dt(&mut self, sample_dt: f32) {
		self.alpha = Self::calc_alpha(self.freq, sample_dt);
	}

	fn calc_alpha(freq: f32, dt: f32) ->  f32 {
		dt / (dt + (TAU * freq).recip())
	}

	fn evaluate(&mut self, value: f32) -> f32 {
		self.prev_value = self.prev_value + (value - self.prev_value) * self.alpha;
		self.prev_value
	}
}
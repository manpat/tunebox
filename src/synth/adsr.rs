
#[derive(Copy, Clone, Debug)]
enum State {
	Silence, Attack, Decay, Sustain, Release,
}

#[derive(Clone, Debug)]
pub struct Adsr {
	state: State,
	position: f32,

	gate: Gate,

	gain_lvl: f32,

	atk_inc: f32,
	dec_inc: f32,
	sus_lvl: f32,
	rel_inc: f32,
}

impl Adsr {
	pub fn new(atk: f32, dec: f32, sus_lvl: f32, rel: f32, gain: f32) -> Adsr {
		let sus_lvl = sus_lvl.max(0.0).min(1.0);

		Adsr {
			state: State::Silence,
			position: 0.0,

			gate: Gate::new(),
			gain_lvl: gain.sqrt(),

			// NOTE: this model doesn't allow decay to be cancelled on gate falling edge
			// this may or may not be desirable but needs thought
			atk_inc: 1.0 / atk.max(0.00001),
			dec_inc: (1.0 - sus_lvl) / dec.max(0.00001),
			sus_lvl,
			rel_inc: sus_lvl / rel.max(0.00001),
		}
	}

	pub fn is_silent(&self) -> bool {
		matches!(self.state, State::Silence)
	}

	pub fn set_gain(&mut self, gain: f32) {
		self.gain_lvl = gain.sqrt();
	}

	pub fn value(&self) -> f32 {
		self.position
	}

	fn update(&mut self, gate: GateState, dt: f32) {
		use self::State::*;

		self.state = match self.state {
			Silence => if gate.is_rising_edge() {
				self.position = 0.0;
				Attack
			} else {
				Silence
			}

			Attack => {
				self.position += self.atk_inc * dt;

				if self.position >= self.gain_lvl {
					self.position = self.position.min(1.0);
					Decay
				} else {
					Attack
				}
			}

			Decay => {
				self.position -= self.dec_inc * dt;

				if gate.is_rising_edge() {
					Attack
				} else if self.position <= self.sus_lvl*self.gain_lvl {
					self.position = self.position.max(self.sus_lvl*self.gain_lvl);
					Sustain
				} else {
					Decay
				}
			}

			Sustain => if gate.is_lowish() {
				Release
			} else if gate.is_rising_edge() {
				Attack
			} else {
				Sustain
			}

			Release => {
				self.position -= self.rel_inc * dt;

				if gate.is_rising_edge() {
					Attack
				} else if self.position <= 0.0 {
					self.position = 0.0;
					Silence
				} else {
					Release
				}
			}
		}
	}

	pub fn advance(&mut self, sample_dt: f32, value: bool) -> f32 {
		let sample = self.position;
		let gate = self.gate.update(value);
		self.update(gate, sample_dt);
		sample * sample
	}
}




#[derive(Clone, Debug)]
pub struct Gate(GateState);

impl Gate {
	pub fn new() -> Self { Gate (GateState::Low) }

	pub fn update(&mut self, new_value: bool) -> GateState {
		use self::GateState::*;

		self.0 = match self.0 {
			Low => {
				if new_value { RisingEdge }
				else { Low }
			}

			RisingEdge => {
				if new_value { High }
				else { FallingEdge }
			}

			High => {
				if !new_value { FallingEdge }
				else { High }
			}

			FallingEdge => {
				if !new_value { Low }
				else { RisingEdge }
			}
		};

		self.0
	}
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum GateState { Low, RisingEdge, High, FallingEdge }

impl GateState {
	pub fn is_rising_edge(self) -> bool {
		match self {
			GateState::RisingEdge => true,
			_ => false
		}
	}
	pub fn is_falling_edge(self) -> bool {
		match self {
			GateState::FallingEdge => true,
			_ => false
		}
	}
	pub fn is_highish(self) -> bool {
		match self {
			GateState::RisingEdge => true,
			GateState::High => true,
			_ => false
		}
	}
	pub fn is_lowish(self) -> bool {
		match self {
			GateState::FallingEdge => true,
			GateState::Low => true,
			_ => false
		}
	}
}
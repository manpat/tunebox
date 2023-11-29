use toybox::prelude::*;

use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::{Arc, Mutex, MutexGuard};

mod adsr;
mod provider;
use provider::*;

pub use provider::{UiFeedback, UiVoice};


#[derive(Clone)]
pub struct SynthController {
	msg_tx: SyncSender<SynthMessage>,

	ui_feedback: Option<Arc<Mutex<UiFeedback>>>,
}

impl SynthController {
	pub fn note_on(&self, note: u8, velocity: u8) {
		self.send(SynthMessage::NoteOn{note, velocity});
	}

	pub fn note_off(&self, note: u8) {
		self.send(SynthMessage::NoteOff(note));
	}

	pub fn enable_ui_feedback(&mut self) {
		let ui_feedback = Arc::new(Mutex::new(UiFeedback {
			voices: Vec::with_capacity(16)
		}));

		self.ui_feedback = Some(Arc::clone(&ui_feedback));
		self.send(SynthMessage::SetUiFeedback(Some(ui_feedback)));
	}

	pub fn ui_feedback(&self) -> Option<MutexGuard<'_, UiFeedback>> {
		self.ui_feedback.as_ref()
			.and_then(|m| m.lock().ok())
	}

	fn send(&self, msg: SynthMessage) {
		self.msg_tx.send(msg).unwrap();
	}
}


pub fn init_synth(audio: &mut audio::System) -> anyhow::Result<SynthController> {
	let (msg_tx, msg_rx) = sync_channel(128);
	audio.set_provider(SynthProvider::new(msg_rx))?;
	Ok(SynthController {
		msg_tx,
		ui_feedback: None,
	})
}

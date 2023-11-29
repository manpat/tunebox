use toybox::prelude::*;

use std::sync::mpsc::{sync_channel, SyncSender};

mod adsr;
mod provider;
use provider::*;



#[derive(Clone)]
pub struct SynthController {
	msg_tx: SyncSender<SynthMessage>,
}

impl SynthController {
	pub fn note_on(&self, note: u8, velocity: u8) {
		self.msg_tx.send(SynthMessage::NoteOn{note, velocity}).unwrap();
	}

	pub fn note_off(&self, note: u8) {
		self.msg_tx.send(SynthMessage::NoteOff(note)).unwrap();
	}
}


pub fn init_synth(audio: &mut audio::System) -> anyhow::Result<SynthController> {
	let (msg_tx, msg_rx) = sync_channel(128);
	audio.set_provider(SynthProvider::new(msg_rx))?;
	Ok(SynthController {msg_tx})
}

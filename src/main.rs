#![feature(slice_as_chunks)]

use toybox::prelude::*;

mod synth;
use synth::*;


fn main() -> anyhow::Result<()> {
	toybox::run("tunebox", App::new)
}


struct App {
	controller: SynthController,
	_midi_connection: midir::MidiInputConnection<()>,
}


impl App {
	fn new(ctx: &mut toybox::Context) -> anyhow::Result<App> {
		let controller = init_synth(&mut ctx.audio)?;

		Ok(App {
			_midi_connection: start_midi(controller.clone())?,
			controller,
		})
	}
}


impl toybox::App for App {
	fn present(&mut self, ctx: &mut toybox::Context) {
		egui::CentralPanel::default()
			.show(&ctx.egui, |ui| {
				ui.label("Hello");
			});
	}
}



fn start_midi(controller: SynthController) -> anyhow::Result<midir::MidiInputConnection<()>> {
	let mut midi_in = midir::MidiInput::new("Input")?;
	midi_in.ignore(midir::Ignore::None);

	let ports = midi_in.ports();
	if ports.is_empty() {
		anyhow::bail!("No midi ports!");
	}

	for port in ports.iter() {
		println!("port: {}", midi_in.port_name(port)?);
	}

	midi_in.connect(
		&ports[0],
		"tunebox-input",
		move |_stamp, message_raw, _| {
			if let Ok((msg, _)) = midi_msg::MidiMsg::from_midi(&message_raw) {
				process_midi_event(msg, &controller);
			}
		},
		()
	)
	.map_err(Into::into)
}

fn process_midi_event(msg: midi_msg::MidiMsg, controller: &SynthController) {
	use midi_msg::*;

	match msg {
		MidiMsg::ChannelVoice{ msg: ChannelVoiceMsg::NoteOn{note, velocity: 0}, .. }
		| MidiMsg::ChannelVoice{ msg: ChannelVoiceMsg::NoteOff{note, ..}, .. }
		=> {
			controller.note_off(note);
		}

		MidiMsg::ChannelVoice{ msg: ChannelVoiceMsg::NoteOn{note, velocity}, .. } => {
			controller.note_on(note, velocity);
		}

		_ => {}
	}
}
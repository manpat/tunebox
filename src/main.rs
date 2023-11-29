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
		let mut controller = init_synth(&mut ctx.audio)?;
		controller.enable_ui_feedback();

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
				ui.horizontal_wrapped(|ui| {
					let Some(ui_feedback) = self.controller.ui_feedback() else {
						println!("NONE!");
						return
					};

					for voice in ui_feedback.voices.iter() {
						let (response, painter) = ui.allocate_painter(egui::vec2(64.0, 64.0 + 16.0), egui::Sense::hover());
						let (rect, pan_rect) = response.rect
							.split_top_bottom_at_y(response.rect.top() + 64.0);

						let margin = 5.0;
						let rounding = 5.0;
						let outer_rounding = margin + rounding;

						let (_, env_rect) = rect
							.shrink(margin)
							.split_top_bottom_at_fraction(1.0 - voice.envelope.sqrt());

						let env_color = match voice.active {
							true => egui::Color32::GRAY,
							false => egui::Color32::DARK_GRAY,
						};

						let stroke_color = match voice.active {
							true => egui::Color32::WHITE,
							false => egui::Color32::GRAY,
						};

						painter.rect_filled(rect, outer_rounding, stroke_color);
						painter.rect_filled(rect.shrink(2.0), outer_rounding - 2.0, ui.style().visuals.window_fill);

						painter.rect_filled(env_rect, rounding, env_color);

						let pan_ramped = if voice.pan > 0.0 {
							1.0 - (1.0 - voice.pan).powi(2)
						} else {
							(1.0 + voice.pan).powi(2) - 1.0
						};

						let pan_rect = pan_rect.shrink2(egui::vec2(0.0, 4.0));
						let extent_x = pan_rect.width() / 2.0 * pan_ramped;

						let scaled_pan_rect = if extent_x.abs() < rounding {
							egui::Rect::from_center_size(pan_rect.center(), egui::vec2(rounding, pan_rect.height()))
						} else if extent_x > 0.0 {
							egui::Rect::from_min_size(pan_rect.center_top(), egui::vec2(extent_x, pan_rect.height()))
						} else {
							egui::Rect::from_min_size(pan_rect.center_top() + egui::vec2(extent_x, 0.0), egui::vec2(-extent_x, pan_rect.height()))
						};

						painter.rect_filled(pan_rect, rounding, egui::Color32::DARK_GRAY);
						painter.rect_filled(scaled_pan_rect, rounding, egui::Color32::GRAY);

						// painter.rect_stroke(rect, outer_rounding, egui::Stroke::new(2.0, stroke_color));

						painter.text(
							rect.center(),
							egui::Align2::CENTER_CENTER,
							format!("{}", voice.note),
							egui::FontId::proportional(18.0),
							stroke_color
						);
					}
				});
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
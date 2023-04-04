use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::Arc;

use typst::doc::Frame;

pub fn spawn(egui_ctx: egui::Context) -> (SyncSender<String>, Receiver<Result<Frame, String>>) {
	let sandbox = Arc::new(crate::sandbox::Sandbox::new());

	let (in_send, in_recv) = std::sync::mpsc::sync_channel(4);
	let (out_send, out_recv) = std::sync::mpsc::sync_channel(4);

	std::thread::spawn(move || {
		while let Ok(input) = in_recv.recv() {
			let compiled = typst::compile(&Arc::clone(&sandbox).with_source(input))
				.map_err(|errors| format!("{errors:#?}"))
				.and_then(|doc| {
					doc
						.pages
						.into_iter()
						.next()
						.ok_or_else(|| "no frames".into())
				});
			_ = out_send.send(compiled);
			egui_ctx.request_repaint();
		}
	});

	(in_send, out_recv)
}

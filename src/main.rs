use std::sync::mpsc::{Receiver, SyncSender};

use eframe::egui;
use egui::{pos2, vec2, Color32, Pos2, Rect, Rounding, Sense, Stroke};
use typst::doc::{Frame, FrameItem, GroupItem};
use typst::geom::{Abs, Geometry, Paint, PathItem, Point, Shape};

mod renderer;
mod sandbox;

fn main() {
	let native_options = eframe::NativeOptions::default();
	eframe::run_native(
		"My egui App",
		native_options,
		Box::new(|cc| Box::new(App::new(cc))),
	)
	.unwrap();
}

struct App {
	renderer: SyncSender<String>,
	rendered: Receiver<Result<Frame, String>>,

	buffer: String,
	current_frame: Option<Result<Frame, String>>,
}

impl App {
	fn new(cc: &eframe::CreationContext<'_>) -> Self {
		let (renderer, rendered) = renderer::spawn(cc.egui_ctx.clone());
		Self {
			renderer,
			rendered,
			buffer: String::new(),
			current_frame: None,
		}
	}
}

const PIXELS_PER_POINT: f32 = 1.25;

fn to_px(abs: Abs) -> f32 {
	abs.to_pt() as f32 * PIXELS_PER_POINT
}

impl eframe::App for App {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		egui::CentralPanel::default().show(ctx, |ui| {
			if let Ok(rendered) = self.rendered.try_recv() {
				self.current_frame = Some(rendered);
			}

			ui.text_edit_multiline(&mut self.buffer);

			if ui.button("Render").clicked() {
				self.renderer.send(self.buffer.clone()).unwrap();
			}

			match &self.current_frame {
				None => {
					ui.label("please render");
				}
				Some(Err(error)) => {
					ui.label(format!("Errors:\n{error}"));
				}
				Some(Ok(frame)) => {
					egui::Frame::canvas(&ctx.style()).show(ui, |ui| {
						let (rect, _response) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
						let painter = ui.painter().with_clip_rect(rect);
						render(&painter, frame);
					});
				}
			}
		});
	}
}

fn translate_paint(paint: Paint) -> Color32 {
	match paint {
		Paint::Solid(color) => {
			let typst::geom::RgbaColor { r, g, b, a } = color.to_rgba();
			// XXX is it premultiplied?
			Color32::from_rgba_unmultiplied(r, g, b, a)
		}
	}
}

fn render_item(painter: &egui::Painter, origin: Pos2, position: Point, item: &FrameItem) {
	let translate_point = |point: Point| origin + vec2(to_px(point.x), to_px(point.y));
	let translate_size = |size: typst::geom::Size| vec2(to_px(size.x), to_px(size.y));
	let translate_stroke = |stroke: Option<typst::geom::Stroke>| {
		stroke.map_or(Stroke::NONE, |stroke| {
			(to_px(stroke.thickness), translate_paint(stroke.paint)).into()
		})
	};

	let position = translate_point(position);
	match item {
		FrameItem::Group(GroupItem {
			frame,
			transform,
			clips,
		}) => {
			assert!(
				transform.is_identity(),
				"non-identity transforms not yet implemented"
			);
			let inner_painter = if *clips {
				painter.with_clip_rect(Rect::from_min_size(position, translate_size(frame.size())))
			} else {
				painter.clone()
			};
			render(&inner_painter, frame);
		}
		FrameItem::Text(..) => todo!(),
		FrameItem::Shape(
			Shape {
				geometry,
				fill,
				stroke,
			},
			_span,
		) => match geometry {
			Geometry::Line(to_point) => painter.line_segment(
				[position, translate_point(*to_point)],
				translate_stroke(stroke.as_ref().cloned()),
			),
			Geometry::Rect(size) => painter.rect(
				Rect::from_min_size(position, translate_size(*size)),
				Rounding::none(),
				fill
					.as_ref()
					.cloned()
					.map_or(Color32::TRANSPARENT, translate_paint),
				translate_stroke(stroke.as_ref().cloned()),
			),
			Geometry::Path(..) => todo!(),
		},
		FrameItem::Image(..) => todo!(),
		FrameItem::Meta(..) => {}
	}
}

fn render_inner<'a>(
	painter: &egui::Painter,
	origin: Pos2,
	items: impl Iterator<Item = &'a (Point, FrameItem)>,
) {
	for (position, item) in items {
		render_item(painter, origin, *position, item);
	}
}

fn render(painter: &egui::Painter, frame: &Frame) {
	let origin = painter.clip_rect().left_top();
	let items = frame.items();
	render_inner(painter, origin, items);
}

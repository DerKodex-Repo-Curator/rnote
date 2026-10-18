// Imports
use crate::engine::EngineView;
use p2d::bounding_volume::Aabb;
use vello_cpu::RenderContext;

/// Trait for types that can draw themselves on a [RenderContext].
pub trait Drawable {
    /// Draw itself.
    ///
    /// The implementers are expected to save/restore the drawing context state themselves.
    /// `image_scale` is the scale-factor of generated images within the type.
    /// The content should not be zoomed by it!
    fn draw(&self, cx: &mut RenderContext, image_scale: f64) -> anyhow::Result<()>;

    /// Draw itself to a [cairo::Context].
    fn draw_to_cairo(
        &self,
        cx: &cairo::Context,
        width: u16,
        height: u16,
        image_scale: f64,
    ) -> anyhow::Result<()> {
        let mut vello_target = vello_cpu::Pixmap::new(width, height);
        let mut vello_resources = vello_cpu::Resources::new();
        let mut vello_cx = RenderContext::new(width, height);
        self.draw(&mut vello_cx, image_scale)?;
        vello_cx.flush();
        vello_cx.render(&mut vello_target, &mut vello_resources);

        let mut cairo_surface = cairo::ImageSurface::create_for_data(
            vello_target.data_as_u8_slice().to_vec(),
            cairo::Format::ARgb32,
            width as i32,
            height as i32,
            4 * width as i32,
        )?;
        cx.set_source_surface(&mut cairo_surface, 0., 0.)?;
        cx.paint()?;
        Ok(())
    }
}

/// Trait for types that can draw themselves on the document.
///
/// In the coordinate space of the document.
pub trait DrawableOnDoc {
    /// Bounds on the document.
    fn bounds_on_doc(&self, engine_view: &EngineView) -> Option<Aabb>;

    /// Draw itself on the document.
    ///
    /// The implementers are expected to save/restore the drawing context state themselves.
    fn draw_on_doc(&self, cx: &mut RenderContext, engine_view: &EngineView) -> anyhow::Result<()>;

    /// Draw itself on the snapshot.
    ///
    /// The snapshot is expected to be untransformed in surface coordinate space.
    #[cfg(feature = "ui")]
    fn draw_on_doc_to_gtk_snapshot(
        &self,
        snapshot: &gtk4::Snapshot,
        engine_view: &EngineView,
    ) -> anyhow::Result<()> {
        use crate::ext::GrapheneRectExt;
        use gtk4::{gdk, graphene, prelude::*};
        use rnote_compose::ext::{AabbExt, DAffine2Ext};

        if let Some(bounds) = self.bounds_on_doc(engine_view) {
            snapshot.save();

            let viewport = engine_view.camera.viewport();
            // Restrict to viewport as maximum bounds, else cairo is very unperformant
            // and will even crash for very large bounds
            let bounds = bounds.clamp(None, Some(viewport));

            let mut bounds_on_surface = bounds
                .scale(engine_view.camera.total_zoom())
                .translate(-engine_view.camera.offset())
                .ceil();
            bounds_on_surface.ensure_positive();
            bounds_on_surface.assert_valid()?;
            let width = bounds_on_surface.extents().x.ceil() as u16;
            let height = bounds_on_surface.extents().y.ceil() as u16;

            let mut vello_target = vello_cpu::Pixmap::new(width, height);
            let mut vello_resources = vello_cpu::Resources::new();
            let mut vello_cx = RenderContext::new(width, height);
            vello_cx.set_transform(engine_view.camera.transform().to_kurbo());
            self.draw_on_doc(&mut vello_cx, engine_view)?;
            vello_cx.flush();
            vello_cx.render(&mut vello_target, &mut vello_resources);

            let memory_texture = gdk::MemoryTexture::new(
                width as i32,
                height as i32,
                gdk::MemoryFormat::R8g8b8a8Premultiplied,
                &glib::Bytes::from(vello_target.data_as_u8_slice()),
                4 * width as usize,
            );
            snapshot.append_texture(
                &memory_texture,
                &graphene::Rect::from_p2d_aabb(bounds_on_surface),
            );

            snapshot.restore();
        }

        Ok(())
    }
}

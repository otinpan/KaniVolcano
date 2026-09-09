use anyhow::{Result, ensure};
use cosmic_text::{
    Attrs, Buffer, FontSystem, Metrics, Shaping, Wrap,
};

pub(crate) fn layout_text(
    font_system: &mut FontSystem,
    text: &str,
    attrs: &Attrs<'_>,
    font_size: f32,
    line_height: f32,
) -> Result<Buffer> {
    ensure!(
        font_size.is_finite() && font_size > 0.0,
        "font size must be finite and positive"
    );
    ensure!(
        line_height.is_finite() && line_height > 0.0,
        "line height must be finite and positive"
    );

    let mut buffer = Buffer::new(
        font_system,
        Metrics::new(font_size, line_height),
    );

    buffer.set_size(None, None);
    buffer.set_wrap(Wrap::None);

    buffer.set_text(
        text,
        attrs,
        Shaping::Advanced,
        None,
    );

    buffer.shape_until_scroll(font_system, false);

    Ok(buffer)
}
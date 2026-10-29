use crate::palette::{PolineManager, PolineManagerImpl};

pub const DEFAULT_HUE_OFFSET_DEGREES: f64 = 171.0;
pub const HUE_STORAGE_KEY: &str = "liquid-hue-v2";
pub const HUE_CHANGE_EVENT: &str = "liquid-hue-change";

fn srgb_channel_to_linear(channel: u8) -> f64 {
    let channel = channel as f64 / 255.0;
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

pub fn relative_luminance([r, g, b]: [u8; 3]) -> f64 {
    0.2126 * srgb_channel_to_linear(r)
        + 0.7152 * srgb_channel_to_linear(g)
        + 0.0722 * srgb_channel_to_linear(b)
}

fn canvas_display_channel(channel: u8) -> u8 {
    // WGPU samples palette colors as floats and writes them to an sRGB
    // surface. Mirror that encoding for static CSS backgrounds.
    let channel = channel as f64 / 255.0;
    let encoded = if channel <= 0.003_130_8 {
        12.92 * channel
    } else {
        1.055 * channel.powf(1.0 / 2.4) - 0.055
    };

    (encoded.clamp(0.0, 1.0) * 255.0).round() as u8
}

pub fn canvas_display_color([r, g, b]: [u8; 3]) -> [u8; 3] {
    [
        canvas_display_channel(r),
        canvas_display_channel(g),
        canvas_display_channel(b),
    ]
}

fn contrast_ratio(a: [u8; 3], b: [u8; 3]) -> f64 {
    let a = relative_luminance(a);
    let b = relative_luminance(b);
    let (lighter, darker) = if a >= b { (a, b) } else { (b, a) };

    (lighter + 0.05) / (darker + 0.05)
}

pub fn canvas_background_color(colors: &[[u8; 3]]) -> [u8; 3] {
    colors
        .get(colors.len() / 2)
        .copied()
        .map(canvas_display_color)
        .unwrap_or([0, 0, 0])
}

pub fn most_contrasting_palette_color(colors: &[[u8; 3]], background: [u8; 3]) -> Option<[u8; 3]> {
    colors.iter().copied().max_by(|a, b| {
        contrast_ratio(*a, background)
            .partial_cmp(&contrast_ratio(*b, background))
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}

pub fn readable_palette_color(colors: &[[u8; 3]]) -> [u8; 3] {
    most_contrasting_palette_color(colors, canvas_background_color(colors))
        .unwrap_or([255, 255, 255])
}

pub fn code_block_background_color(colors: &[[u8; 3]]) -> [u8; 3] {
    most_contrasting_palette_color(colors, canvas_background_color(colors)).unwrap_or([31, 41, 55])
}

pub fn rgb_css([r, g, b]: [u8; 3]) -> String {
    format!("rgb({r}, {g}, {b})")
}

pub fn palette_for_slider(hue: f64) -> PolineManagerImpl {
    PolineManagerImpl::new(hue + DEFAULT_HUE_OFFSET_DEGREES)
}

pub fn canvas_background_style(poline: &PolineManagerImpl) -> String {
    format!(
        "background: {};",
        rgb_css(canvas_background_color(poline.colors()))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slider_palette_uses_the_original_default_offset() {
        let palette = palette_for_slider(0.0);
        assert_eq!(*palette.abs_hue(), DEFAULT_HUE_OFFSET_DEGREES);
        assert_eq!(palette.colors().len(), 258);
    }
}
